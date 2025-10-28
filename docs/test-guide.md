# 🧪 dyapix 测试指南

本文档提供了 dyapix 代理服务器的基本测试流程，包括路由和上游数据的初始化、访问测试以及增量更新测试。

## 测试环境

- **代理端口**: 8080 (开发环境)
- **上游服务**: httpbin.org
- **数据库**: MySQL (localhost:3306/dyapix)

## 1. 初始化路由和上游数据

### 1.1 创建上游数据

首先在数据库中插入一个上游配置，使用 httpbin.org 作为后端服务：

```sql
INSERT INTO dyapix_ds (`key`, ds_type, ds_json, ds_status, operation_type) VALUES 
('upstream-httpbin', 'upstream', '{
  "id": "upstream-httpbin",
  "name": "HTTPBin Upstream",
  "desc": "Test upstream using httpbin.org",
  "type": "roundrobin",
  "scheme": "https",
  "nodes": {
    "httpbin.org:443": 1
  },
  "retries": 2,
  "timeout": {
    "connect": 10,
    "send": 10,
    "read": 10
  }
}', 'pending', 'create');
```

### 1.2 创建路由数据

创建路由配置，将请求转发到 httpbin.org：

```sql
INSERT INTO dyapix_ds (`key`, ds_type, ds_json, ds_status, operation_type) VALUES 
('route-test', 'route', '{
  "id": "route-test",
  "name": "Test Route",
  "desc": "Test route for httpbin.org",
  "priority": 100,
  "uris": ["/test/*"],
  "methods": ["GET", "POST", "PUT", "DELETE"],
  "hosts": ["localhost"],
  "plugins": {},
  "upstream_id": "upstream-httpbin"
}', 'pending', 'create');
```

### 1.3 验证数据同步

检查数据是否已同步到代理服务器：

```sql
SELECT `key`, ds_type, ds_status, operation_type, create_time 
FROM dyapix_ds 
WHERE is_deleted = 0 
ORDER BY create_time DESC;
```

## 2. 访问路由测试

### 2.1 启动代理服务器

```bash
cd /Users/nndaphne/Documents/豆芽APP/开源/dyapix
cargo run --bin dyapix
```

### 2.2 测试基本路由访问

#### 测试 GET 请求
```bash
curl -X GET "http://localhost:8080/test/get" \
  -H "Host: localhost" \
  -v
```

**预期结果**: 返回 httpbin.org 的响应，包含请求信息

#### 测试 POST 请求
```bash
curl -X POST "http://localhost:8080/test/post" \
  -H "Host: localhost" \
  -H "Content-Type: application/json" \
  -d '{"test": "data"}' \
  -v
```

**预期结果**: 返回 httpbin.org 的响应，包含 POST 数据和请求头信息

#### 测试 PUT 请求
```bash
curl -X PUT "http://localhost:8080/test/put" \
  -H "Host: localhost" \
  -H "Content-Type: application/json" \
  -d '{"update": "test"}' \
  -v
```

#### 测试 DELETE 请求
```bash
curl -X DELETE "http://localhost:8080/test/delete" \
  -H "Host: localhost" \
  -v
```

### 2.3 测试路由匹配

#### 测试 URI 匹配
```bash
# 应该匹配
curl -X GET "http://localhost:8080/test/anything" \
  -H "Host: localhost" \
  -v

# 不应该匹配（返回 404）
curl -X GET "http://localhost:8080/other/path" \
  -H "Host: localhost" \
  -v
```

#### 测试 Host 匹配
```bash
# 应该匹配
curl -X GET "http://localhost:8080/test/get" \
  -H "Host: localhost" \
  -v

# 不应该匹配（返回 404）
curl -X GET "http://localhost:8080/test/get" \
  -H "Host: otherhost.com" \
  -v
```

## 3. 增量路由和上游数据

### 3.1 添加新的上游

添加第二个上游节点，实现负载均衡：

```sql
UPDATE dyapix_ds 
SET ds_json = '{
  "id": "upstream-httpbin",
  "name": "HTTPBin Upstream",
  "desc": "Test upstream using httpbin.org with multiple nodes",
  "type": "roundrobin",
  "scheme": "https",
  "nodes": {
    "httpbin.org:443": 1,
    "httpbin.org:443": 1
  },
  "retries": 2,
  "timeout": {
    "connect": 10,
    "send": 10,
    "read": 10
  }
}', 
ds_status = 'pending', 
operation_type = 'update'
WHERE `key` = 'upstream-httpbin';
```

### 3.2 添加新的路由

添加一个高优先级的路由：

```sql
INSERT INTO dyapix_ds (`key`, ds_type, ds_json, ds_status, operation_type) VALUES 
('route-test-high-priority', 'route', '{
  "id": "route-test-high-priority",
  "name": "High Priority Test Route",
  "desc": "High priority route for specific test paths",
  "priority": 200,
  "uris": ["/test/special/*"],
  "methods": ["GET"],
  "hosts": ["localhost"],
  "plugins": {},
  "upstream_id": "upstream-httpbin"
}', 'pending', 'create');
```

### 3.3 添加内联上游的路由

创建一个使用内联上游配置的路由：

```sql
INSERT INTO dyapix_ds (`key`, ds_type, ds_json, ds_status, operation_type) VALUES 
('route-inline-upstream', 'route', '{
  "id": "route-inline-upstream",
  "name": "Inline Upstream Route",
  "desc": "Route with inline upstream configuration",
  "priority": 50,
  "uris": ["/inline/*"],
  "methods": ["GET", "POST"],
  "hosts": ["localhost"],
  "plugins": {},
  "upstream": {
    "retries": 1,
    "timeout": {
      "connect": 5,
      "send": 5,
      "read": 5
    },
    "type": "roundrobin",
    "scheme": "https",
    "nodes": {
      "httpbin.org:443": 1
    }
  }
}', 'pending', 'create');
```

## 4. 访问路由测试（增量更新后）

### 4.1 测试高优先级路由

```bash
# 测试高优先级路由
curl -X GET "http://localhost:8080/test/special/anything" \
  -H "Host: localhost" \
  -v
```

**预期结果**: 应该匹配高优先级路由（priority: 200）

### 4.2 测试内联上游路由

```bash
# 测试内联上游路由
curl -X GET "http://localhost:8080/inline/get" \
  -H "Host: localhost" \
  -v

curl -X POST "http://localhost:8080/inline/post" \
  -H "Host: localhost" \
  -H "Content-Type: application/json" \
  -d '{"inline": "test"}' \
  -v
```

**预期结果**: 使用内联配置的上游，超时时间更短（5秒）

### 4.3 测试路由优先级

```bash
# 测试路由优先级 - 应该匹配高优先级路由
curl -X GET "http://localhost:8080/test/special/priority-test" \
  -H "Host: localhost" \
  -v

# 测试普通路由 - 应该匹配普通优先级路由
curl -X GET "http://localhost:8080/test/regular-path" \
  -H "Host: localhost" \
  -v
```

### 4.4 测试负载均衡

多次请求测试负载均衡效果：

```bash
for i in {1..5}; do
  echo "Request $i:"
  curl -s "http://localhost:8080/test/get" \
    -H "Host: localhost" | jq '.origin'
  sleep 1
done
```

**预期结果**: 如果配置了多个节点，应该看到不同的 origin IP

## 5. 测试验证点

### 5.1 功能验证

- ✅ 路由匹配正确（URI、Host、Method）
- ✅ 上游转发正常
- ✅ 负载均衡工作
- ✅ 优先级路由生效
- ✅ 内联上游配置生效
- ✅ 增量更新生效

### 5.2 性能验证

- ✅ 响应时间合理（< 1秒）
- ✅ 并发请求处理正常
- ✅ 内存使用稳定

### 5.3 错误处理验证

- ✅ 404 错误（不匹配的路由）
- ✅ 上游服务不可用时的处理
- ✅ 超时处理

## 6. 清理测试数据

测试完成后清理数据：

```sql
-- 删除测试路由
UPDATE dyapix_ds 
SET is_deleted = 1, operation_type = 'delete', ds_status = 'pending'
WHERE `key` IN ('route-test', 'route-test-high-priority', 'route-inline-upstream');

-- 删除测试上游
UPDATE dyapix_ds 
SET is_deleted = 1, operation_type = 'delete', ds_status = 'pending'
WHERE `key` = 'upstream-httpbin';
```

## 7. 故障排查

### 7.1 常见问题

1. **路由不匹配**
   - 检查 URI 模式是否正确
   - 确认 Host 头设置
   - 验证 HTTP 方法

2. **上游连接失败**
   - 检查网络连接
   - 验证上游服务可用性
   - 检查超时配置

3. **数据同步问题**
   - 检查数据库连接
   - 验证 ds_status 状态
   - 查看代理服务器日志

### 7.2 日志查看

```bash
# 查看应用日志
tail -f logs/app.log.$(date +%Y-%m-%d)

# 查看代理服务器日志
cargo run --bin dyapix 2>&1 | tee proxy.log
```

---

**注意**: 本测试指南基于开发环境配置，生产环境请相应调整配置参数。
