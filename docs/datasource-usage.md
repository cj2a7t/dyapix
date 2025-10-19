# Datasource 使用指南

## 📚 快速开始

### 1. 初始化数据库

```bash
# 连接到 MySQL 数据库
mysql -u your_user -p your_database < db/init.sql
```

### 2. 配置数据库连接

在配置文件中设置 MySQL 连接信息：

```toml
[data_source.mysql]
url = "mysql://user:password@localhost:3306/dyapix"
max_connections = 10
min_connections = 2
acquire_timeout_secs = 30
idle_timeout_secs = 600
max_lifetime_secs = 1800
test_before_acquire = true
```

### 3. 启动数据源监听

#### 方式一：动态数据源（推荐）⭐

根据配置文件自动选择数据源类型：

```rust
use dyapix_common::datasource::{get_datasource, DynamicDataSource};
use dyapix_common::datasource::interface::DataSource;
use dyapix_common::datasource::mysql::{init_shutdown_channel, trigger_shutdown};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化关闭通道
    let shutdown_tx = init_shutdown_channel();
    
    // 根据配置自动创建数据源（推荐方式）
    let datasource = get_datasource().await?;
    
    tracing::info!("Using datasource: {}", datasource.datasource_type());
    
    // 启动数据源监听（这会阻塞当前任务）
    let handle = tokio::spawn(async move {
        if let Err(e) = datasource.fetch_and_watch().await {
            tracing::error!("Datasource watcher failed: {}", e);
        }
    });
    
    // ... 应用主逻辑 ...
    
    // 优雅关闭
    trigger_shutdown();
    handle.await?;
    
    Ok(())
}
```

#### 方式二：直接指定数据源类型

如果需要直接指定特定的数据源：

```rust
use dyapix_common::datasource::DynamicDataSource;
use dyapix_common::datasource::interface::DataSource;
use dyapix_common::datasource::mysql::MysqlDataSource;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 方式 2a: 从配置创建（自动选择）
    let datasource = DynamicDataSource::from_config().await?;
    
    // 方式 2b: 手动创建 MySQL 数据源
    let datasource = DynamicDataSource::Mysql(MysqlDataSource);
    
    // ... 使用 datasource ...
    
    Ok(())
}
```

#### 配置文件示例

在 `config/default.toml` 中配置：

```toml
[app]
data_source = "mysql"  # 或 "etcd"（未来支持）

[data_source.mysql]
url = "mysql://user:password@localhost:3306/dyapix"
max_connections = 10
min_connections = 2
acquire_timeout_secs = 30
idle_timeout_secs = 600
max_lifetime_secs = 1800
test_before_acquire = true
```

---

## 🔧 CRUD 操作

### 创建或更新资源

```rust
use dyapix_common::datasource::{get_datasource, interface::DataSource};
use dyapix_common::cro::Route;

async fn create_or_update_route() -> anyhow::Result<()> {
    // 获取全局数据源实例
    let datasource = get_datasource().await?;
    
    let route = Route {
        id: "route-001".to_string(),
        // ... 其他字段
    };
    
    // 插入或更新路由
    let result = datasource.put(&route).await?;
    println!("Route saved: {:?}", result);
    
    Ok(())
}
```

### 获取单个资源

```rust
async fn get_route() -> anyhow::Result<()> {
    let datasource = get_datasource().await?;
    
    // 通过 ID 获取路由
    let route: Route = datasource.get("route-001").await?;
    println!("Found route: {:?}", route);
    
    Ok(())
}
```

### 获取所有资源

```rust
async fn get_all_routes() -> anyhow::Result<()> {
    let datasource = get_datasource().await?;
    
    // 获取所有路由
    let routes: Vec<Route> = datasource.get_all().await?;
    println!("Total routes: {}", routes.len());
    
    for route in routes {
        println!("- {}: {}", route.id, route.path);
    }
    
    Ok(())
}
```

### 删除资源

```rust
async fn delete_route() -> anyhow::Result<()> {
    let datasource = get_datasource().await?;
    
    // 软删除路由
    let deleted: bool = datasource.delete::<Route>("route-001").await?;
    
    if deleted {
        println!("Route deleted successfully");
    } else {
        println!("Route not found or already deleted");
    }
    
    Ok(())
}
```

---

## 🏥 健康检查

### 检查数据源健康状态

```rust
use dyapix_common::datasource::mysql::MysqlDataSource;

async fn check_health() -> anyhow::Result<()> {
    let status = MysqlDataSource::health_check().await;
    
    println!("Health Status: {:?}", status);
    println!("- Healthy: {}", status.healthy);
    println!("- Pool Size: {}/{}", status.pool_status.size, status.pool_status.max_size);
    println!("- Idle Connections: {}", status.pool_status.idle);
    println!("- Pending Records: {}", status.pending_count);
    println!("- Syncing Records: {}", status.syncing_count);
    
    if let Some(error) = status.error {
        println!("- Error: {}", error);
    }
    
    Ok(())
}
```

### 获取统计信息

```rust
async fn get_statistics() -> anyhow::Result<()> {
    let stats = MysqlDataSource::get_stats().await?;
    
    println!("Statistics:");
    println!("- Total Records: {}", stats.total_count);
    println!("- Active Records: {}", stats.active_count);
    println!("- Deleted Records: {}", stats.deleted_count);
    println!("- Synced Records: {}", stats.synced_count);
    
    Ok(())
}
```

---

## 🔌 扩展新的资源类型

### 1. 定义 CRO 资源

```rust
use serde::{Deserialize, Serialize};
use dyapix_common::cro::CRO;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCert {
    pub id: String,
    pub domain: String,
    pub cert_pem: String,
    pub key_pem: String,
}

impl CRO for TlsCert {
    fn cro_kind() -> &'static str {
        "TlsCert"
    }
    
    fn id(&self) -> &str {
        &self.id
    }
}
```

### 2. 实现 CROHandler

```rust
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use dyapix_common::datasource::mysql::handler::{CROEntity, CROHandler};
use dyapix_common::cache::CacheEventType;

// 扩展 CROEntity 枚举
// 注意：这需要修改 handler.rs 中的 CROEntity 定义
// pub enum CROEntity {
//     Route(Box<Route>),
//     Upstream(Upstream),
//     TlsCert(TlsCert),  // 新增
// }

pub struct TlsCertHandler;

#[async_trait]
impl CROHandler for TlsCertHandler {
    fn parse_entity(&self, json: &str) -> Result<CROEntity> {
        let cert: TlsCert = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse TlsCert from JSON: {}", e))?;
        Ok(CROEntity::TlsCert(cert))
    }
    
    async fn insert_into_cache(
        &self,
        operation_type: &str,
        entity: CROEntity,
        _prev_entity: Option<CROEntity>,
    ) -> bool {
        let cert = match entity {
            CROEntity::TlsCert(c) => c,
            _ => {
                tracing::error!("Expected TlsCert entity, got different type");
                return false;
            }
        };
        
        // 实现缓存更新逻辑
        match operation_type {
            "create" | "update" => {
                // 更新 TLS 证书缓存
                // tls_cache::insert(cert).await;
                true
            }
            "delete" => {
                // 从缓存中删除
                // tls_cache::remove(&cert.id).await;
                true
            }
            _ => {
                tracing::error!("Unknown operation_type: {}", operation_type);
                false
            }
        }
    }
}
```

### 3. 注册 Handler

在 `handler.rs` 的 `CROHandlerRegistry::global()` 中注册新的 handler：

```rust
registry.register(
    CRO_KIND_TLS_CERT,
    Box::new(TlsCertHandler),
);
```

---

## 📊 监控和调试

### 日志级别

建议的日志配置：

```rust
// 开发环境
tracing_subscriber::fmt()
    .with_env_filter("dyapix_common::datasource=debug")
    .init();

// 生产环境
tracing_subscriber::fmt()
    .with_env_filter("dyapix_common::datasource=info")
    .init();
```

### 关键日志事件

- **初始加载：** `Starting initial full load of datasource records...`
- **监听启动：** `Entering watch loop for pending datasource records...`
- **同步成功：** `✓ Synced record: id = {id}, key = {key}`
- **同步失败：** `✗ Failed to sync record, resetting to pending: id = {id}`
- **优雅关闭：** `Received shutdown signal, stopping watcher...`

### 性能监控

建议监控的指标：

```rust
// 可以使用 Prometheus 或其他指标系统
metrics::gauge!("datasource.pending_count", status.pending_count as f64);
metrics::gauge!("datasource.syncing_count", status.syncing_count as f64);
metrics::gauge!("datasource.pool.size", status.pool_status.size as f64);
metrics::gauge!("datasource.pool.idle", status.pool_status.idle as f64);
```

---

## ⚠️ 注意事项

### 1. 并发删除

如果记录正在同步中（`ds_status = 'syncing'`），删除操作会失败并返回错误：

```rust
use dyapix_common::datasource::{get_datasource, interface::DataSource};

async fn safe_delete() -> anyhow::Result<()> {
    let datasource = get_datasource().await?;
    
    match datasource.delete::<Route>("route-001").await {
        Ok(true) => println!("Deleted successfully"),
        Ok(false) => println!("Record not found"),
        Err(e) if e.to_string().contains("syncing") => {
            println!("Record is being synced, please retry later");
            // 可以实现重试逻辑
        }
        Err(e) => println!("Delete failed: {}", e),
    }
    
    Ok(())
}
```

### 2. 数据库连接池

连接池会在首次使用时自动初始化，确保配置正确：

```rust
// 如果连接失败，会返回详细错误信息
match get_mysql_pool().await {
    Ok(pool) => println!("Pool initialized"),
    Err(e) => eprintln!("Failed to connect to MySQL: {}", e),
}
```

### 3. 缓存同步延迟

从数据库写入到缓存生效有轻微延迟（取决于轮询间隔，默认 5 秒）：

```rust
// 写入数据库
datasource.put(&route).await?;

// 此时缓存可能还未更新，需要等待 watcher 处理
// 建议：如果需要立即生效，可以直接操作缓存
```

---

## 🚀 最佳实践

### 1. 使用全局数据源实例（推荐）⭐

```rust
use dyapix_common::datasource::{get_datasource, interface::DataSource};

async fn example() -> anyhow::Result<()> {
    // 推荐：使用全局单例，避免重复创建
    let datasource = get_datasource().await?;
    
    // 所有操作都使用同一个实例
    datasource.put(&route).await?;
    datasource.get::<Route>("route-001").await?;
    
    Ok(())
}
```

**优点：**
- ✅ 单例模式，全局唯一实例
- ✅ 自动根据配置选择数据源类型
- ✅ 线程安全，可以在多个任务间共享
- ✅ 避免重复初始化

### 2. 配置驱动的数据源选择

```toml
# 开发环境：使用 MySQL
[app]
data_source = "mysql"

# 未来可以轻松切换到其他数据源
# data_source = "etcd"
# data_source = "redis"
```

**好处：**
- ✅ 不需要修改代码，只需改配置
- ✅ 不同环境可以使用不同的数据源
- ✅ 便于测试和部署

### 3. 启动时初始化关闭通道

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志
    tracing_subscriber::fmt::init();
    
    // 2. 初始化关闭通道
    let shutdown_tx = init_shutdown_channel();
    
    // 3. 启动数据源监听
    let datasource = get_datasource().await?;
    tokio::spawn(async move {
        if let Err(e) = datasource.fetch_and_watch().await {
            tracing::error!("Datasource failed: {}", e);
        }
    });
    
    // 4. 启动应用服务
    // ...
    
    // 5. 等待关闭信号
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");
    
    // 6. 触发优雅关闭
    trigger_shutdown();
    
    Ok(())
}
```

### 4. 错误处理要完善

```rust
async fn handle_route_update(route: Route) -> anyhow::Result<()> {
    let datasource = get_datasource().await?;
    
    match datasource.put(&route).await {
        Ok(result) => {
            tracing::info!("Route updated successfully: {}", result.id);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to update route {}: {}", route.id, e);
            // 根据错误类型进行不同处理
            if e.to_string().contains("connection") {
                // 数据库连接问题，可能需要重试
                Err(anyhow::anyhow!("Database connection error: {}", e))
            } else {
                // 其他错误
                Err(e)
            }
        }
    }
}
```

### 5. 定期监控健康状态

```rust
use std::time::Duration;
use dyapix_common::datasource::mysql::MysqlDataSource;

async fn start_health_monitor() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            
            let status = MysqlDataSource::health_check().await;
            
            if !status.healthy {
                tracing::error!("❌ Datasource unhealthy: {:?}", status.error);
            } else {
                tracing::debug!(
                    "✅ Datasource healthy - pending: {}, syncing: {}, pool: {}/{}",
                    status.pending_count,
                    status.syncing_count,
                    status.pool_status.size,
                    status.pool_status.max_size
                );
            }
        }
    });
}
```

### 6. 支持多数据源切换

如果需要同时支持多种数据源（例如测试时使用 Mock）：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_with_mock_datasource() {
        // 测试环境可以使用不同的数据源
        let datasource = DynamicDataSource::Mysql(MysqlDataSource);
        
        // 或者实现 MockDataSource
        // let datasource = DynamicDataSource::Mock(MockDataSource::new());
        
        // 执行测试...
    }
}
```

---

