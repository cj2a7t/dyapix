CREATE TABLE IF NOT EXISTS dyapix_ds (
    id BIGINT PRIMARY KEY AUTO_INCREMENT, 
    `key` VARCHAR(255) NOT NULL UNIQUE,
    ds_type VARCHAR(64) NOT NULL COMMENT 'Data source type: route, upstream, tls',
    ds_json TEXT NOT NULL,
    prev_ds_json TEXT NULL COMMENT 'Previous ds_json before update',
    ds_status ENUM('pending', 'syncing', 'synced') NOT NULL DEFAULT 'pending',
    operation_type ENUM('create', 'update', 'delete') NOT NULL DEFAULT 'create' COMMENT 'Operation type: create, update, delete',
    is_deleted TINYINT(1) NOT NULL DEFAULT 0 COMMENT 'Logical delete flag: 0-not deleted, 1-deleted',
    create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    update_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;