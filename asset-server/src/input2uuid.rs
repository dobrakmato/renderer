use crate::models::Asset;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub async fn dump_input2uuid(input2uuid_file: &str, assets: Vec<Asset>) {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&input2uuid_file)
        .await
        .unwrap();

    for x in assets {
        let x: Asset = x;

        file.write_all(
            format!("{}={}\n", x.name(), x.uuid().to_hyphenated().to_string()).as_bytes(),
        )
        .await
        .unwrap();
    }
}
