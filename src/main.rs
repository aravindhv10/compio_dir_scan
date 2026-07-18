#[compio::main]
async fn main() -> std::io::Result<()> {
    let tmp: Vec<String> = std::env::args().collect();
    for i in 1..tmp.len() {
    let res = compio::fs::read("./main.rs").await?;
    }

    eprintln!("{:?}", res);

    Ok(())
}
