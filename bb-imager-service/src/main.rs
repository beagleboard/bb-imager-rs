use std::{collections::HashMap, error::Error, future::pending};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zbus::{connection, fdo, interface, message::Header, object_server::SignalEmitter};
use zbus_polkit::policykit1;

struct Pb2Mspm0 {
    conn: connection::Connection,
}

#[interface(name = "org.beagleboard.ImagingService.Pocketbeagle2Mspm0v1")]
impl Pb2Mspm0 {
    const AUTH_ACTION_ID: &str = "org.beagleboard.ImagingService.Pocketbeagle2Mspm0.authn";

    async fn device(&self) -> bb_flasher_pb2_mspm0::Device {
        bb_flasher_pb2_mspm0::device()
    }

    /// Check if the sysfs entries are in order. Also useful for escalating privileges early.
    async fn check(&self, #[zbus(header)] hdr: Header<'_>) -> fdo::Result<()> {
        check_authorization(&self.conn, hdr).await?;

        bb_flasher_pb2_mspm0::check()
            .await
            .map_err(|_| fdo::Error::Failed("Cannot find mspm0".to_string()))
    }

    /// Flash MSPM0
    async fn flash(
        &self,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        firmware: Vec<u8>,
        persist_eeprom: bool,
    ) -> fdo::Result<()> {
        check_authorization(&self.conn, hdr).await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel::<bb_flasher_pb2_mspm0::Status>(20);

        let task = tokio::spawn(async move {
            bb_flasher_pb2_mspm0::flash(&firmware, &tx, persist_eeprom).await
        });

        while let Some(s) = rx.recv().await {
            let msg = serde_json::to_string(&s).unwrap();
            let _ = emitter.status(&msg).await;
        }

        task.await
            .unwrap()
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    #[zbus(signal)]
    /// Signal providing flashing status as JSON string
    async fn status(signal_emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;
}

async fn check_authorization(conn: &connection::Connection, hdr: Header<'_>) -> fdo::Result<()> {
    let proxy = policykit1::AuthorityProxy::new(conn).await?;

    let subject = policykit1::Subject::new_for_message_header(&hdr)
        .map_err(|_| fdo::Error::Failed("Failed to construct polkit subject".to_string()))?;
    let r = proxy
        .check_authorization(
            &subject,
            Pb2Mspm0::AUTH_ACTION_ID,
            &HashMap::new(),
            policykit1::CheckAuthorizationFlags::AllowUserInteraction.into(),
            "",
        )
        .await?;

    if r.is_authorized {
        Ok(())
    } else {
        Err(fdo::Error::AuthFailed(
            "Authetication with polkit failed".to_string(),
        ))
    }
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .expect("Failed to register tracing_subscriber");

    let pb2_mspm0 = Pb2Mspm0 {
        conn: connection::Connection::system().await?,
    };
    let _conn = connection::Builder::system()?
        .name("org.beagleboard.ImagingService")?
        .serve_at(
            "/org/beagleboard/ImagingService/Pocketbeagle2Mspm0v1",
            pb2_mspm0,
        )?
        .build()
        .await?;

    tracing::info!("Started Service");

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
