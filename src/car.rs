use sqlx::{query, query_as, PgPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Car {
    id: i32,
    brand: String,
    model: String,
    year: i32,
}

pub async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create the cars table if it doesn't exist
    query(
        r#"
        CREATE TABLE IF NOT EXISTS car (
            id SERIAL PRIMARY KEY,
            brand VARCHAR(50) NOT NULL,
            model VARCHAR(50) NOT NULL,
            year INT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_car(pool: &PgPool, brand: &str, model: &str, year: i32) -> Result<Car, sqlx::Error> {
    query_as::<_, Car>(
        r#"
        INSERT INTO car (brand, model, year)
        VALUES ($1, $2, $3)
        RETURNING id, brand, model, year;
        "#,
    )
    .bind(brand)
    .bind(model)
    .bind(year)
    .fetch_one(pool)
    .await
}

pub async fn get_car(pool: &PgPool, id: i64) -> Result<Car, sqlx::Error> {
    query_as::<_, Car>(
        r#"
        SELECT id, brand, model, year
        FROM car
        WHERE id = $1;
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn all_cars(pool: &PgPool) -> Result<Vec<Car>, sqlx::Error> {
    query_as::<_, Car>(
        r#"
        SELECT id, brand, model, year
        FROM car;
        "#,
    )
    .fetch_all(pool)
    .await
}   

pub async fn update_car(pool: &PgPool, id: i64, brand: &str, model: &str, year: i32) -> Result<Option<Car>, sqlx::Error> {
    query_as::<_, Car>(
        r#"
        UPDATE car
        SET brand = $1, model = $2, year = $3
        WHERE id = $4
        RETURNING id, brand, model, year;
        "#,
    )
    .bind(brand)
    .bind(model)
    .bind(year)
    .bind(id)
    .fetch_optional(pool)
    .await
}   

pub async fn delete_car(pool: &PgPool, id: i64) -> Result<u64, sqlx::Error> {
    let rows_affected = query(
        r#"
        DELETE FROM car
        WHERE id = $1;
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}   

pub async fn car_main() -> Result<(), sqlx::Error> {
    let database_url = "postgres://postgres:aniruddha@localhost/learning"; // Update with your database URL
    let pool = PgPool::connect(database_url).await?;

     // Initialize the database schema
    init_db(&pool).await?;

    // Example operations
    let new_car = create_car(&pool, "Toyota", "Corolla", 2020).await?;
    println!("Created Car: {:?}", new_car);

    let fetched_car = get_car(&pool, 1).await?;
    println!("Fetched Car: {:?}", fetched_car);

    let all_cars = all_cars(&pool).await?;
    println!("All Cars: {:?}", all_cars);

    // let updated_car = update_car(&pool, new_car.id, "Toyota", "Camry", 2021).await?;
    // println!("Updated Car: {:?}", updated_car);

    // let deleted_count = delete_car(&pool, new_car.id).await?;
    // println!("Deleted Cars Count: {}", deleted_count);

    Ok(())
}