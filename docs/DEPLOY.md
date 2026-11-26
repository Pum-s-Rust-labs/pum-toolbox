# Hướng dẫn Deploy Miễn Phí — GHCR + Render (Singapore)

`pum-toolbox` dùng kiến trúc **GitHub Actions tự build Docker image → đẩy lên `ghcr.io` → Render kéo image về chạy trong ~15 giây** (100% miễn phí, không cần thẻ tín dụng, tránh hoàn toàn lỗi thiếu RAM khi biên dịch Rust trên Free Tier).

## Bước 1: Chờ GitHub Actions build xong image lần đầu

Mỗi khi push lên nhánh `main`, workflow `.github/workflows/release-image.yml` tự động build `Dockerfile` (React SPA + binary Rust `pum-api`) và đẩy lên:

```text
ghcr.io/pum-s-rust-labs/pum-toolbox:latest
```

> **Lưu ý quan trọng lần đầu**: Vào trang tổ chức `https://github.com/orgs/Pum-s-Rust-labs/packages` → chọn package `pum-toolbox` → **Package settings** → **Change visibility** → chuyển sang **Public** (nếu mặc định đang là Private) để Render kéo image không cần token.

## Bước 2: Tạo Web Service miễn phí trên Render

1. Đăng nhập https://render.com bằng tài khoản GitHub (`Pum283`).
2. Chọn **New + → Web Service → Existing Image** (hoặc **New + → Blueprint** và chọn repo `Pum-s-Rust-labs/pum-toolbox` để đọc `render.yaml`).
3. Nếu chọn **Existing Image**:
   - **Image URL**: `ghcr.io/pum-s-rust-labs/pum-toolbox:latest`
   - **Region**: `Singapore (Southeast Asia)`
   - **Instance Type**: `Free`
   - **Health Check Path**: `/health`
4. Điền các **Environment Variables**:
   - `APP_HOST` = `0.0.0.0`
   - `COOKIE_SECURE` = `true`
   - `ALLOW_PRIVATE_MONITOR_TARGETS` = `false`
   - `MAX_UPLOAD_MB` = `10`
   - `JWT_SECRET` = chuỗi ngẫu nhiên dài ít nhất 32 ký tự
   - `DATABASE_URL` = chuỗi PostgreSQL (nhớ đổi ký tự `+` trong mật khẩu thành `%2B`)
   - `PUBLIC_BASE_URL` = `https://<ten-service>.onrender.com`
   - `CORS_ALLOWED_ORIGINS` = `https://<ten-service>.onrender.com`
   - `BOOTSTRAP_ADMIN_EMAIL` = `admin@pumlabs.dev`
   - `BOOTSTRAP_ADMIN_USERNAME` = `superadmin`
   - `BOOTSTRAP_ADMIN_PASSWORD` = mật khẩu mạnh của bạn
5. Nhấn **Deploy Web Service** — sau ~15–30 giây web sẽ online với HTTPS miễn phí!

## Bước 3 (Tuỳ chọn): Tự động redeploy mỗi lần push code mới

1. Trên Render, mở **Settings → Deploy Hook**, sao chép URL Deploy Hook.
2. Vào GitHub repo `Pum-s-Rust-labs/pum-toolbox` → **Settings → Secrets and variables → Actions → New repository secret**:
   - Name: `RENDER_DEPLOY_HOOK_URL`
   - Value: dán URL Deploy Hook từ Render.
3. Từ đó trở đi, mỗi lần `git push main`: GitHub Actions tự build image mới lên `ghcr.io` rồi gọi thẳng Render cập nhật bản mới tự động!

## Bước 4 (Tuỳ chọn): Giữ thức cho Uptime Monitor (Free 24/7)

Render Free cung cấp **750 giờ/tháng** (đủ chạy 24/7 cho 1 web service), nhưng sẽ tự ngủ sau 15 phút không có request. Bạn có thể tạo 1 job miễn phí trên https://cron-job.org gọi `GET https://<ten-service>.onrender.com/health` mỗi **10 phút** để giữ worker Uptime Monitor hoạt động liên tục.
