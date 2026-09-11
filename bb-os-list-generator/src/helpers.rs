use reqwest::{Client, StatusCode, header::RANGE};
use url::Url;

const XZ_FOOTER_LEN: u64 = 12;

pub(crate) async fn xz_extract_size(client: &Client, url: &Url, size: u64) -> anyhow::Result<u64> {
    if size < XZ_FOOTER_LEN {
        anyhow::bail!("file too small")
    }

    let footer = fetch_range(client, url, size - XZ_FOOTER_LEN, size - 1).await?;
    if footer.len() != XZ_FOOTER_LEN as usize || &footer[10..12] != b"YZ" {
        anyhow::bail!("Invalid footer")
    }

    // Backward Size is stored as `real_size / 4 - 1`.
    let backward_size = u32::from_le_bytes(footer[4..8].try_into()?);
    let backward_size = (u64::from(backward_size) + 1) * 4;
    let index_start = size
        .checked_sub(XZ_FOOTER_LEN + backward_size)
        .ok_or(anyhow::anyhow!("xz too small"))?;

    let index = fetch_range(client, url, index_start, size - XZ_FOOTER_LEN - 1).await?;
    parse_xz_index(&index)
}

async fn fetch_range(
    client: &reqwest::Client,
    url: &Url,
    start: u64,
    end: u64,
) -> anyhow::Result<Vec<u8>> {
    let resp = client
        .get(url.clone())
        .header(RANGE, format!("bytes={start}-{end}"))
        .send()
        .await?;

    // A 200 here means the server ignored Range and is about to hand us the
    // whole multi-GB image; bail rather than stream it.
    if resp.status() != StatusCode::PARTIAL_CONTENT {
        anyhow::bail!("Server cannot do partitial content")
    }

    Ok(resp.bytes().await?.to_vec())
}

fn parse_xz_index(buf: &[u8]) -> anyhow::Result<u64> {
    let mut pos = 0usize;
    if buf[0] != 0x00 {
        anyhow::bail!("Invalid xz index")
    }
    pos += 1;

    let (num_recs, temp) =
        read_varint(&buf[pos..]).ok_or(anyhow::anyhow!("Failed to read varint"))?;
    pos += temp;
    let mut total = 0u64;
    for _ in 0..num_recs {
        let (_unpadded, temp) =
            read_varint(&buf[pos..]).ok_or(anyhow::anyhow!("Failed to read varint"))?;
        pos += temp;
        let (uncompressed_size, temp) =
            read_varint(&buf[pos..]).ok_or(anyhow::anyhow!("Failed to read varint"))?;
        pos += temp;
        total = total
            .checked_add(uncompressed_size)
            .ok_or(anyhow::anyhow!(""))?;
    }
    Ok(total)
}

fn read_varint(buf: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    for (i, &byte) in buf.iter().take(9).enumerate() {
        value |= u64::from(byte & 0x7f) << (i * 7);
        if byte & 0x80 == 0 {
            // The spec forbids non-minimal encodings: a trailing 0x00
            // continuation byte that contributes nothing.
            if byte == 0 && i != 0 {
                return None;
            }
            return Some((value, i + 1));
        }
    }
    None // ran off the end, or more than 9 bytes
}
