# 📝 dyapix Core Resources Object Overview

This document introduces the core components of the dyapix: **Route**, **TLS (SSL Certificate)**, and **Upstream**. It also explains how Upstreams can be configured directly within a Route.

---

## 1. Route

A **Route** defines how client requests are matched and forwarded to upstream services.
It supports URI, Host, HTTP method matching, priority, labels, and plugin configuration. Upstreams can be referenced via `upstream_id` or configured inline within the route.

**Example with inline Upstream:**

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

**Key Points:**

- **Matching**: Host → URI → HTTP Method
- **Priority**: Higher value takes precedence when multiple routes match
- **Labels**: Custom metadata for categorization or routing logic
- **Plugins**: Optional functionality such as rate limiting, logging, authentication
- **Upstream**: Can be referenced via `upstream_id` or defined inline within the route

---

## 2. TLS / SSL Certificate

TLS (Transport Layer Security) ensures secure HTTPS connections.
Certificates can be applied per domain using **SNI** (Server Name Indication).

**Example:**

```json
{
  "id": "1",
  "cert": "-----BEGIN CERTIFICATE----- ... -----END CERTIFICATE-----",
  "key": "-----BEGIN PRIVATE KEY----- ... -----END PRIVATE KEY-----",
  "snis": ["dyapix.io"]
}
```

**Key Points:**

- **SNI**: Matches the domain requested by the client
- **Cert/Key**: PEM format public certificate and private key
- Supports multiple SNIs for a single certificate

---

## 3. Upstream

An **Upstream** defines the backend services that handle proxied requests.
It supports multiple nodes, load balancing, retries, and timeout settings.
Upstreams can be defined separately and referenced by routes or configured directly within a route.

**Example:**

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

**Key Points:**

- **Nodes**: List of backend servers with weights
- **Load Balancing**: Round-robin, least connections, or custom algorithms
- **Retries**: Number of retry attempts on failure
- **Timeouts**: Connection, send, and read timeouts
- **Scheme**: HTTP or HTTPS protocol for upstream requests
- **Inline Upstream**: Can be configured directly in the Route instead of referencing an upstream ID

---

This overview provides the essential concepts to configure routes, secure connections with TLS, and define upstream backends in the dyapix.
