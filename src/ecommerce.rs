use sqlx::query;
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Debug, Clone)]
struct NewOrderItem {
    sku: String,
    qty: i32,
}

pub async fn ecom_main() -> Result<(), sqlx::Error> {
    let pool = PgPool::connect("postgres://postgres:aniruddha@localhost/learning").await?;

    // init_db(&pool).await?;
    // seed_data(&pool).await?;

    // 2) Create an order for customer_id=1 with a few SKUs
    let items = vec![
        NewOrderItem { sku: "SKU-USB-C".into(), qty: 2 },
        NewOrderItem { sku: "SKU-KBD-61".into(), qty: 1 },
    ];

    let order_id = create_order_with_items(&pool, 1, items).await?;
    println!("\n✅ Created order id = {}\n", order_id);

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
    let order_id: i64 =
        sqlx::query_scalar("INSERT INTO sales_order (customer_id) VALUES ($1) RETURNING id")
            .bind(customer_id)
            .fetch_one(&mut *tx)
            .await?;

    // 2) For each item:
    //    - decrement inventory if enough stock (single UPDATE..RETURNING)
    //    - insert order_item with captured unit price
    for it in items {
        // decrement stock and get the current price at the same time
        let maybe_price: Option<i32> = sqlx::query_scalar(
            r#"
            UPDATE inventory
               SET on_hand = on_hand - $2
             WHERE sku = $1
               AND on_hand >= $2
            RETURNING price_cents
            "#,
        )
        .bind(&it.sku)
        .bind(it.qty)
        .fetch_optional(&mut *tx)
        .await?;

        let unit_price = match maybe_price {
            Some(p) => p,
            None => {
                // insufficient stock → rollback & error
                tx.rollback().await?;
                return Err(sqlx::Error::RowNotFound);
            }
        };
        println!("SKU={} x{} @ ₹{:.2}  ", it.sku, it.qty, (unit_price as f64) / 100.0);

        // insert order_item
        sqlx::query(
            r#"
            INSERT INTO order_item (order_id, sku, qty, unit_price_cents)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(order_id)
        .bind(&it.sku)
        .bind(it.qty)
        .bind(unit_price)
        .execute(&mut *tx)
        .await?;
    }

    // 3) JOIN **inside the transaction**: compute a summary of the order
    //    (so if we roll back later, this view would also be rolled back)
    let (oid, customer_name, total_cents): (i64, String, i64) = sqlx::query_as(
        r#"
        SELECT so.id AS order_id,
               c.name AS customer_name,
               COALESCE(SUM(oi.qty * oi.unit_price_cents), 0) AS total_cents
          FROM sales_order so
          JOIN customer c    ON c.id = so.customer_id
          LEFT JOIN order_item oi ON oi.order_id = so.id
         WHERE so.id = $1
         GROUP BY so.id, c.name
        "#,
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await?;

    println!("-- Tx JOIN (order summary) -----------------------------");
    println!(
        "order_id={oid}, customer={customer_name}, total=₹{:.2}",
        (total_cents as f64) / 100.0
    );

    // 4) JOIN lines: each item joined with inventory to show title + line total
    let line_rows: Vec<(String, String, i32, i32)> = sqlx::query_as(
        r#"
        SELECT oi.sku,
               i.title,
               oi.qty,
               (oi.qty * oi.unit_price_cents) AS line_cents
          FROM order_item oi
          JOIN inventory i ON i.sku = oi.sku
         WHERE oi.order_id = $1
         ORDER BY i.title
        "#,
    )
    .bind(order_id)
    .fetch_all(&mut *tx)
    .await?;

    println!("-- Tx JOIN (line details) ------------------------------");
    for (sku, title, qty, line_cents) in line_rows {
        println!(
            "{sku:>10}  {:<28}  x{qty:<2}  line=₹{:.2}",
            title,
            (line_cents as f64) / 100.0
        );
    }
    println!("--------------------------------------------------------\n");

    // 5) Commit: all changes (order, items, inventory) become visible
    tx.commit().await?;

    Ok(order_id)
}
