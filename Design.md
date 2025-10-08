┌─────────────────────────────────────────────┐
│ CLI (journal_cli)                           │
│ - 管理文章 (CRUD)                            │
│ - 處理 markdown + 下載圖片                   │
│ - 需要 API_BASE_URL 環境變數                 │
└─────────────────────────────────────────────┘
                    ↓ 寫入
┌─────────────────────────────────────────────┐
│ PostgreSQL Database                         │
│ - posts table                               │
│ - post_assets table                         │
└─────────────────────────────────────────────┘
                    ↑ 讀取
┌─────────────────────────────────────────────┐
│ Web API (journal-core)                      │
│ - 只提供讀取端點                             │
│ - GET /api/posts                            │
│ - GET /api/posts/{uuid}                     │
│ - GET /api/assets/{uuid}                    │
│ - 不需要 API_BASE_URL                        │
└─────────────────────────────────────────────┘
                    ↑ HTTP
┌─────────────────────────────────────────────┐
│ Frontend UI                                 │
│ - 讀取並顯示文章                             │
│ - 渲染 markdown                              │
└─────────────────────────────────────────────┘



journal-core/
├── Cargo.toml
├── .env
├── src/
│   ├── lib.rs                    # 共用的 library
│   ├── bin/
│   │   ├── api.rs                # Web API 主程式
│   │   └── cli.rs                # CLI 主程式
│   │
│   ├── common/                   # 共用模組
│   │   ├── mod.rs
│   │   ├── db.rs                 # 資料庫連接
│   │   └── models.rs             # 資料模型
│   │
│   ├── api/                      # API 專用
│   │   ├── mod.rs
│   │   └── handlers/
│   │       ├── mod.rs
│   │       ├── post_handler.rs   # 文章查詢
│   │       └── asset_handler.rs  # 資源查詢
│   │
│   └── cli/                      # CLI 專用
│       ├── mod.rs
│       ├── commands.rs           # CLI 命令定義
│       └── markdown_processor.rs # Markdown 處理（CLI 專用）
│
└── static/
    └── uploads/                  # 上傳檔案目錄