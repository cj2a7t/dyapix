# 📝 dyapix 核心资源对象概览

本文档介绍了 dyapix 的核心组件：**路由（Route）**、**TLS（SSL 证书）**和**上游（Upstream）**，并说明上游可以直接在路由中配置。

---

## 1. 路由（Route）

**Route**定义了客户端请求如何匹配并转发到上游服务。
它支持 URI、Host、HTTP 方法匹配、优先级、标签（Label）以及插件（Plugin）配置。上游可以通过 `upstream_id` 引用，也可以直接在路由中内联配置。

**示例（内联上游配置）：**

```json
{
  "id": "1",
  "name": "Route xxx",
  "desc": "hello world",
  "priority": 0,
  "label": {
    "key1": "value1"
  },
  "uris": ["/a", "/b"],
  "methods": ["GET", "POST"],
  "hosts": ["a.dyapix.io", "b.dyapix.io"],
  "plugins": {},
  "upstream": {
    "retries": 1,
    "timeout": {
      "connect": 15,
      "send": 15,
      "read": 15
    },
    "type": "roundrobin",
    "scheme": "http",
    "nodes": {
      "up1.dyapix.io:80": 1,
      "up2.dyapix.io:80": 2
    }
  }
}
```

**关键点：**

- **匹配顺序**：Host → URI → HTTP 方法
- **优先级**：当多个路由匹配时，数值越高优先级越高
- **标签（Label）**：自定义元数据，用于分类或路由逻辑
- **插件（Plugins）**：可选功能，如限流、日志记录、身份认证
- **上游（Upstream）**：可以通过 `upstream_id` 引用，也可以在路由中内联配置

---

## 2. TLS / SSL 证书

TLS（传输层安全协议）用于确保 HTTPS 连接安全。
证书可以按域名应用，通过 **SNI**（Server Name Indication）实现多域名支持。

**示例：**

```json
{
  "id": "1",
  "cert": "-----BEGIN CERTIFICATE----- ... -----END CERTIFICATE-----",
  "key": "-----BEGIN PRIVATE KEY----- ... -----END PRIVATE KEY-----",
  "snis": ["dyapix.io"]
}
```

**关键点：**

- **SNI**：匹配客户端请求的域名
- **Cert/Key**：PEM 格式的公钥证书和私钥
- 支持单个证书配置多个 SNI

---

## 3. 上游（Upstream）

Upstream 定义了处理代理请求的后端服务。
它支持多节点、负载均衡、重试次数和超时设置。
上游可以单独定义，并在路由中引用，也可以直接在路由内联配置。

**示例：**

```json
{
  "id": "1",
  "name": "upstream-xxx",
  "desc": "hello world",
  "type": "roundrobin",
  "scheme": "http",
  "nodes": {
    "up1.dyapix.io:80": 1,
    "up2.dyapix.io:80": 2
  }
}
```

**关键点：**

- **节点（Nodes）**：后端服务器列表及权重
- **负载均衡（Load Balancing）**：轮询（Round-robin）、最少连接或自定义算法
- **重试次数（Retries）**：请求失败后的重试次数
- **超时（Timeouts）**：连接、发送和读取超时
- **协议类型（Scheme）**：HTTP 或 HTTPS
- **内联上游（Inline Upstream）**：可以直接在路由中配置，而不必引用上游 ID

---

本概览提供了配置路由、使用 TLS 保障连接安全以及定义上游后端的核心概念。
