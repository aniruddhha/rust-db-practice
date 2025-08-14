use sqlx::{PgPool, Postgres, Transaction};
use sqlx::{query};

#[derive(Debug, Clone)]
struct NewOrderItem {
    sku: String,
    qty: i32,
}

pub async fn ecom_main() -> Result<(), sqlx::Error> {
    let pool = PgPool::connect("postgres://postgres:aniruddha@localhost/learning").await?;

    init_db(&pool).await?;
    seed_data(&pool).await?;

    // Example usage of creating a new order
    let order_items = vec![
        NewOrderItem {
            sku: "SKU-USB-C".to_string(),
            qty: 2,
        },
        NewOrderItem {
            sku: "SKU-KBD-61".to_string(),
            qty: 1,
        },
    ];
    
    // Here you would typically create a sales order and add items to it

    Ok(())
}

async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    query(
        r#"
        CREATE TABLE IF NOT EXISTS customer (
            id   BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    query(
        r#"
        CREATE TABLE IF NOT EXISTS inventory (
            sku            TEXT PRIMARY KEY,
            title          TEXT NOT NULL,
            price_cents    INT  NOT NULL,
            on_hand        INT  NOT NULL CHECK (on_hand >= 0)
        );
        "#,
    )
    .execute(pool)
    .await?;

    query(
        r#"
        CREATE TABLE IF NOT EXISTS sales_order (
            id           BIGSERIAL PRIMARY KEY,
            customer_id  BIGINT NOT NULL REFERENCES customer(id),
            created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        "#,
    )
    .execute(pool)
    .await?;

    query(
        r#"
        CREATE TABLE IF NOT EXISTS order_item (
            id                BIGSERIAL PRIMARY KEY,
            order_id          BIGINT NOT NULL REFERENCES sales_order(id) ON DELETE CASCADE,
            sku               TEXT   NOT NULL REFERENCES inventory(sku),
            qty               INT    NOT NULL CHECK (qty > 0),
            unit_price_cents  INT    NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    query("INSERT INTO customer(name) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind("Samrudhi")
        .execute(pool)
        .await?;

    let upsert = r#"
        INSERT INTO inventory (sku, title, price_cents, on_hand)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (sku) DO UPDATE
           SET title = EXCLUDED.title,
               price_cents = EXCLUDED.price_cents,
               on_hand = EXCLUDED.on_hand
    "#;

    query(upsert)
        .bind("SKU-USB-C")
        .bind("USB-C Cable (1m)")
        .bind(399) // ₹3.99
        .bind(50)
        .execute(pool)
        .await?;

    query(upsert)
        .bind("SKU-KBD-61")
        .bind("Mechanical Keyboard (61 keys)")
        .bind(4999) // ₹49.99
        .bind(10)
        .execute(pool)
        .await?;

    query(upsert)
        .bind("SKU-IPHONE")
        .bind("Smartphone X")
        .bind(699_00) // ₹699.00
        .bind(2)
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_order_with_items(
    pool: &PgPool,
    customer_id: i64,
    items: Vec<NewOrderItem>,
) -> Result<i64, sqlx::Error> {

    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    // 1) Create the order
    let order_id: i64 = sqlx::query_scalar(
        "INSERT INTO sales_order (customer_id) VALUES ($1) RETURNING id",
    )
    .bind(customer_id)
    .fetch_one(&mut *tx)
    .await?;


    Ok((12))
 }