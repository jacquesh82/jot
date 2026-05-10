use crate::db::Db;
use crate::StorageError;
use chrono::Utc;
use jot_core::models::Device;
use sqlx::Row;
use uuid::Uuid;

fn device_from_row(row: &sqlx::sqlite::SqliteRow) -> Device {
    let id_str: String = row.get("id");
    let identity_str: String = row.get("identity_id");
    let last_seen_str: String = row.get("last_seen");
    Device {
        id: Uuid::parse_str(&id_str).unwrap(),
        identity_id: Uuid::parse_str(&identity_str).unwrap(),
        pub_key_x25519: row.get("pub_key_x25519"),
        pub_key_ed25519: row.get("pub_key_ed25519"),
        name: row.get("name"),
        last_seen: chrono::DateTime::parse_from_rfc3339(&last_seen_str)
            .unwrap()
            .with_timezone(&Utc),
    }
}

impl Db {
    pub async fn insert_device(&self, device: &Device) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO devices (id, identity_id, pub_key_x25519, pub_key_ed25519, name, last_seen)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(device.id.to_string())
        .bind(device.identity_id.to_string())
        .bind(&device.pub_key_x25519)
        .bind(&device.pub_key_ed25519)
        .bind(&device.name)
        .bind(device.last_seen.to_rfc3339())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_device(&self, id: Uuid) -> Result<Option<Device>, StorageError> {
        let row = sqlx::query(
            "SELECT id, identity_id, pub_key_x25519, pub_key_ed25519, name, last_seen FROM devices WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(|r| device_from_row(&r)))
    }

    pub async fn list_devices(&self, identity_id: Uuid) -> Result<Vec<Device>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, identity_id, pub_key_x25519, pub_key_ed25519, name, last_seen FROM devices WHERE identity_id = ?",
        )
        .bind(identity_id.to_string())
        .fetch_all(&self.0)
        .await?;
        Ok(rows.iter().map(device_from_row).collect())
    }

    pub async fn delete_device(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM devices WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn touch_device(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("UPDATE devices SET last_seen = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn rename_device(&self, id: Uuid, name: &str) -> Result<(), StorageError> {
        sqlx::query("UPDATE devices SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id.to_string())
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_db;

    fn make_device(identity_id: Uuid) -> Device {
        Device {
            id: Uuid::new_v4(),
            identity_id,
            pub_key_x25519: "aabbcc".to_string(),
            pub_key_ed25519: "ddeeff".to_string(),
            name: "Test Device".to_string(),
            last_seen: Utc::now(),
        }
    }

    #[tokio::test]
    async fn insert_and_get_round_trip() {
        let db = test_db().await;
        let device = make_device(Uuid::new_v4());
        db.insert_device(&device).await.unwrap();
        let fetched = db.get_device(device.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, device.id);
        assert_eq!(fetched.name, device.name);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let db = test_db().await;
        assert!(db.get_device(Uuid::new_v4()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_devices_filters_by_identity() {
        let db = test_db().await;
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        db.insert_device(&make_device(id1)).await.unwrap();
        db.insert_device(&make_device(id2)).await.unwrap();
        assert_eq!(db.list_devices(id1).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn touch_device_updates_last_seen() {
        let db = test_db().await;
        let device = make_device(Uuid::new_v4());
        db.insert_device(&device).await.unwrap();
        let before = db.get_device(device.id).await.unwrap().unwrap().last_seen;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        db.touch_device(device.id).await.unwrap();
        let after = db.get_device(device.id).await.unwrap().unwrap().last_seen;
        assert!(after >= before);
    }

    #[tokio::test]
    async fn delete_device() {
        let db = test_db().await;
        let device = make_device(Uuid::new_v4());
        db.insert_device(&device).await.unwrap();
        db.delete_device(device.id).await.unwrap();
        assert!(db.get_device(device.id).await.unwrap().is_none());
    }
}
