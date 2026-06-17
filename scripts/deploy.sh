#!/bin/bash
# scripts/deploy.sh

set -e

echo "=== Deploying Bilibili Hot Video Analysis Platform ==="

# 1. 构建爬虫
echo "Building crawler..."
cd crawler
cargo build --release
cd ..

# 2. 构建后端
echo "Building backend..."
cd backend
cargo build --release
cd ..

# 3. 构建前端
echo "Building frontend..."
cd frontend
npm run build
cd ..

# 4. 部署前端
echo "Deploying frontend..."
sudo cp -r frontend/dist/* /var/www/html/

# 5. 部署服务
echo "Deploying services..."
sudo cp scripts/bilibili-crawler.service /etc/systemd/system/
sudo cp scripts/bilibili-backend.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable bilibili-crawler bilibili-backend
sudo systemctl restart bilibili-crawler bilibili-backend

# 6. 部署Nginx配置
echo "Deploying Nginx config..."
sudo cp scripts/nginx.conf /etc/nginx/sites-available/bilibili
sudo ln -sf /etc/nginx/sites-available/bilibili /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx

echo "=== Deployment completed! ==="
