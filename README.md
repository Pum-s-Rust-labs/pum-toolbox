# Pum Toolbox

Nền tảng web **All-in-One Dev & Ops Suite** viết bằng **Rust (Axum + Tokio + SQLx PostgreSQL)** + **React / Vite / TypeScript / Tailwind CSS**, thuộc tổ chức **[Pum's Rust Labs](https://github.com/Pum-s-Rust-labs)**.

## 4 Module tích hợp

1. **Shortlink & Analytics**: Rút gọn link (tự sinh hoặc custom slug), mật khẩu bảo vệ (Argon2id), hạn sử dụng, sinh mã **QR SVG**, redirect `/r/:slug`, thống kê click 14 ngày & theo thiết bị.
2. **Uptime Monitor Real-time**: Theo dõi HTTP/API định kỳ bằng worker bất đồng bộ **Tokio**, đo độ trễ, đẩy cập nhật trực tiếp qua **Server-Sent Events (SSE)** và bảo vệ bằng **SSRF Guard** (chặn IP nội bộ / cloud metadata).
3. **Code & Markdown Pastebin**: Lưu snippet/ghi chú Markdown render HTML đã lọc **XSS (`ammonia`)**, gắn tag, tìm kiếm, hỗ trợ hết hạn theo thời gian và **tự hủy sau N lượt xem (burn-after-reading)**.
4. **ImageLab (Rayon Parallel)**: Xử lý song song đa nhân CPU nhiều ảnh cùng lúc — nén, chuyển đổi định dạng (**WebP / JPEG / PNG**), resize, grayscale, watermark, tự đóng gói `.zip` và bảo vệ chống *decompression bomb* + kiểm tra magic bytes.

## Xác thực, Phân quyền Đa tầng (3-Tier RBAC) & Bảo mật

- **Mật khẩu**: Argon2id kèm salt CSPRNG và chính sách độ mạnh mật khẩu + tự khóa 15 phút sau 5 lần sai liên tiếp.
- **Token 2 lớp**: JWT Access Token ngắn hạn + Refresh Token 256-bit lưu trong cookie `HttpOnly; SameSite=Lax`, xoay vòng token (rotation) và tự thu hồi toàn bộ family khi phát hiện replay attack.
- **Phân quyền đa tầng**:
  - **Tầng 1 (Role Hierarchy)**: `super_admin` > `admin` > `member` > `viewer`.
  - **Tầng 2 (Granular Permissions)**: 13 quyền hạt mịn (`links:read/write/delete_any`, `monitors:...`, `pastes:...`, `images:process`, `users:manage`, `roles:assign`, `audit:read`) hỗ trợ **override riêng cho từng user**.
  - **Tầng 3 (Ownership & Visibility)**: Kiểm soát theo chủ sở hữu `owner_id` và phạm vi `private` / `internal` / `public`.
- **Audit Logging**: Ghi lại đầy đủ thao tác đăng nhập, đổi quyền, khóa tài khoản, tạo/xóa tài nguyên vào bảng `audit_logs`.
- **Security Headers & Rate Limiting**: Sliding-window rate limiter trên `DashMap`, CORS whitelist, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`.

## Cấu trúc dự án

```text
pum-toolbox/
├── migrations/              # SQLx PostgreSQL migrations (tự động chạy khi khởi động)
├── crates/
│   ├── core/                # Domain models, 3-Tier RBAC policy engine, validators
│   ├── security/            # Argon2id, JWT/Refresh, SSRF Guard, XSS Sanitizer, RateLimiter
│   ├── db/                  # PostgreSQL pool & repositories (sqlx)
│   ├── modules/             # Nghiệp vụ Shortlink, Monitor Worker, ImageLab (Rayon/ZIP)
│   └── api/                 # Axum HTTP server, SSE, security middleware, SPA serving
└── web/                     # React + Vite + TypeScript + Tailwind CSS
```

## Hướng dẫn chạy cục bộ

1. Sao chép cấu hình mẫu và điền chuỗi kết nối PostgreSQL:
   ```bash
   cp .env.example .env
   ```
2. Build giao diện frontend (để Axum phục vụ chung trên 1 cổng `8080`):
   ```bash
   cd web
   pnpm install
   pnpm build
   cd ..
   ```
3. Khởi động máy chủ Rust (tự chạy PostgreSQL migrations & tạo tài khoản `SuperAdmin` ban đầu):
   ```bash
   cargo run -p pum-api
   ```
   Mở trình duyệt tại `http://127.0.0.1:8080`.
   *(Khi phát triển frontend với hot-reload, có thể chạy thêm `pnpm dev` trong `web/` tại `http://localhost:5173` — đã cấu hình proxy tự động tới cổng `8080`.)*

## Kiểm tra chất lượng

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd web && pnpm build
```

## Giấy phép

Phát hành theo giấy phép [MIT](LICENSE).
