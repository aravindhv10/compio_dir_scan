fn do_read(inpath: impl AsRef<std::path::Path>) -> Result<Vec<u8>, u8> {
    let inpath = inpath.as_ref();
    match compio::runtime::RuntimeBuilder::new().build() {
        Ok(rt) => {
            let res = rt.block_on(async move { compio::fs::read(inpath).await });
            match res {
                Ok(o) => {
                    return Ok(o);
                }
                Err(e) => {
                    return Err(1);
                }
            };
        }
        Err(e) => {
            return Err(1);
        }
    }
}

#[compio::main]
async fn main() -> std::io::Result<()> {
    Ok(())
}
