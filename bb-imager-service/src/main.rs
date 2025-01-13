use std::{collections::HashMap, error::Error, future::pending};
use zbus::{connection, interface, message::Header};
use zbus_polkit::policykit1;

struct Pb2Mspm0 {
    conn: connection::Connection,
}

#[interface(name = "org.beagleboard.ImagingService.Pocketbeagle2Mspm0")]
impl Pb2Mspm0 {
    const AUTH_ACTION_ID: &str = "org.beagleboard.ImagingService.Pocketbeagle2Mspm0.authn";

    // Can be `async` as well.
    async fn flash(&self, #[zbus(header)] hdr: Header<'_>) -> String {
        let proxy = policykit1::AuthorityProxy::new(&self.conn).await.unwrap();

        let subject = policykit1::Subject::new_for_message_header(&hdr).unwrap();
        let r = proxy
            .check_authorization(
                &subject,
                Self::AUTH_ACTION_ID,
                &HashMap::new(),
                policykit1::CheckAuthorizationFlags::AllowUserInteraction.into(),
                "",
            )
            .await;

        match r {
            Ok(resp) => format!("Autorized: {:?}", resp),
            Err(e) => format!("failed: {:?}", e),
        }
    }
}

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let pb2_mspm0 = Pb2Mspm0 {
        conn: connection::Connection::system().await?,
    };
    let _conn = connection::Builder::system()?
        .name("org.beagleboard.ImagingService")?
        .serve_at(
            "/org/beagleboard/ImagingService/Pocketbeagle2Mspm0",
            pb2_mspm0,
        )?
        .build()
        .await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
