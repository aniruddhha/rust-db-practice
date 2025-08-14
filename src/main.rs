mod car;
use car::car_main;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {

    car_main().await?;

    Ok(())
}

