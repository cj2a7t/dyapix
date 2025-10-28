CREATE TABLE IF NOT EXISTS dyapix_ds (
    id BIGINT PRIMARY KEY AUTO_INCREMENT, 
    `key` VARCHAR(255) NOT NULL UNIQUE,
    ds_type VARCHAR(64) NOT NULL COMMENT 'Data source type: Route, Upstream, TlsCert, etc.',
    ds_json TEXT NOT NULL,
    prev_ds_json TEXT NULL COMMENT 'Previous ds_json before update',
    ds_status ENUM('pending', 'syncing', 'synced') NOT NULL DEFAULT 'pending',
    operation_type ENUM('create', 'update', 'delete') NOT NULL DEFAULT 'create' COMMENT 'Operation type: create, update, delete',
    is_deleted TINYINT(1) NOT NULL DEFAULT 0 COMMENT 'Logical delete flag: 0-not deleted, 1-deleted',
    create_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    update_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    -- Performance optimization indexes
    INDEX idx_ds_status_id (ds_status, id) COMMENT 'For watcher queries (pending records)',
    INDEX idx_ds_type_deleted (ds_type, is_deleted) COMMENT 'For get_all queries',
    INDEX idx_is_deleted (is_deleted) COMMENT 'For health check and statistics'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;