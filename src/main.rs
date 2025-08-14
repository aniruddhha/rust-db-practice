mod car;
use car::car_main;

mod ecommerce;
use ecommerce::ecom_main;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> { ecom_main().await }

