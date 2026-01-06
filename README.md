# Crypto Management Dashboard

A full-stack Rust + React dashboard for managing and visualizing your crypto wallet allocations.

---

## Features

- **Live Price Updates:** Fetches current prices and wallet values from CoinMarketCap.
- **Per-Asset Table:** View all assets with filters for Asset, Group, and BARCA. Includes pagination.
- **Per-Group Table:** See group allocations with current value ($), percent, deviation, and value deviation.
- **BARCA Actual Table:** View actual allocation per BARCA group.
- **Pie Charts:** Visualize allocations by Group and BARCA (target and actual) with readable labels.
- **Tabs:** Switch between tables and charts using tabs.
- **Responsive UI:** Built with Material UI and Recharts, with dark mode enabled.

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js & npm](https://nodejs.org/)
- CoinMarketCap API key (for live prices)

---

## Backend (Rust) Setup

1. **Clone the repository and enter the project directory:**

   ```sh
   git clone <your-repo-url>
   cd crypto_management
   ```

2. **Set up your environment variables:**

   Create a `.env` file in the project root with your CoinMarketCap API key **and your market name**:

   ```
   API_KEY=your_coinmarketcap_api_key
   MARKET=your_market_name
   ```

   - `API_KEY`: Your CoinMarketCap API key.
   - `CURRENT_MARKET`: The market name to use for filtering BARCA targets (e.g., `BullMarket`, `BearMarket`, etc).

3. **Prepare your wallet allocations file:**

   Edit or create `wallet_allocations.csv` in the project root. Example:

   ```
   symbol,group,barca,target_percent,current_quantity
   USDT,Caixa,Caixa,30,1200
   BTC,Holding,Hodl,10,0.75
   ETH,Holding,Hodl,10,2.5
   SOL,Trading,Altcoins,5,10
   ETH,Trading,Altcoins,15,2
   DOGE,Trading,Altcoins,2,1000
   ```

4. **Build and run the backend:**

   ```sh
   cargo run
   ```

   The backend will start at [http://127.0.0.1:3001](http://127.0.0.1:3001).

### Database (SQLite) & migrations

This project supports an optional local SQLite database for history and wallet allocation audit. The repository includes SQL migrations in the `migrations/` folder.

1. Create a folder for the DB and initialize the schema:

```bash
mkdir -p data
# apply the initial migration into a new SQLite file
sqlite3 ./data/crypto.db < migrations/0001_create_history_tables.sql
```

2. Or set `DATABASE_URL` to a file path and let the app connect to it:

```bash
export DATABASE_URL=sqlite://./data/crypto.db
```

Notes:
- The migrations create `history_assets`, `history_barca`, `history_groups`, `history_totals`, the append-only `wallet_allocations` ledger, and an `allocations` table for persisted computed payloads.
- Convenience views power the API:
  - `wallet_allocations_current` now aggregates every wallet row per symbol/group/BARCA (no more “last row wins” bugs).
  - `asset_variance_history`, `barca_variance_history`, and `group_variance_history` expose the dashboard-ready history (value, current %, target %, deviation, value deviation).
- Run `sqlx migrate run` (or start the backend once) whenever the `migrations/` folder changes so the database schema stays in sync.

### Seeding `wallet_allocations`

When the backend starts it checks whether `wallet_allocations_current` is empty and, if so, seeds it from `wallet_allocations.csv` (override the path via `WALLET_ALLOCATIONS_PATH`). You can also trigger the import manually:

- CLI: `cargo run --bin import_wallet_allocations -- wallet_allocations.csv`
- API: `curl -X POST http://127.0.0.1:3001/api/import_wallets -H "Content-Type: application/json" -d '{"path":"wallet_allocations.csv"}'`
- UI: click the **Import Wallet CSV** button next to “Update Prices & Show Distribution”.

5. **Test the API:**

   Visit [http://127.0.0.1:3001/api/allocations](http://127.0.0.1:3001/api/allocations) in your browser or use `curl` to see the JSON output.

---

## Frontend (React) Setup

1. **Navigate to the frontend directory:**

   ```sh
   cd frontend
   ```

2. **Install dependencies:**

   ```sh
   npm install
   ```

3. **Start the React development server:**

   ```sh
   npm run dev
   ```

   The frontend will be available at [http://localhost:5173](http://localhost:5173) (or the port shown in your terminal).

---

## Usage

### Backend

1. **Rust API** (see `src/main.rs`)
   - Reads BARCA targets from `wallet_barca.csv`.
   - Loads wallet positions from the SQLite view `wallet_allocations_current` (auto-seeded from `wallet_allocations.csv`, or import manually through the CLI/UI/API).
   - Serves `/api/allocations` with the computed per-asset/per-group/BARCA breakdowns and persists every snapshot into SQLite for the dashboard.

2. **Run the backend:**
   ```sh
   cargo run
   ```

- Click **"Update Prices & Show Distribution"** in the frontend to fetch live prices and see your portfolio allocation.
- The dashboard displays both per-asset and per-group allocation, with deviations highlighted.
- To update your portfolio, edit `wallet_allocations.csv` and refresh the frontend.

### Importing `wallet_allocations.csv` (manual)

Wallet allocation changes are intentionally manual: edit `wallet_allocations.csv` in the repo root and run the importer to append an audit row into the DB. This avoids noisy duplicate rows when nothing actually changed.

Run:

```bash
# ensure DATABASE_URL is set (default: sqlite://./data/crypto.db)
export DATABASE_URL=sqlite://./data/crypto.db
cargo run --bin import_wallet_allocations -- wallet_allocations.csv
```

This inserts one append-only row per CSV line into `wallet_allocations` (audit/history). Use the view to inspect current values:

```bash
sqlite3 ./data/crypto.db "SELECT * FROM wallet_allocations_current;"
```

---

## Notable Recent Changes

- **Tabs:** All tables and charts are now in tabs for easy navigation.
- **Per-Asset Table:** Added filters for Asset, Group, and BARCA (combinable). Pagination is applied after filtering (20 per page).
- **Per-Group Table:** "Target %" column replaced by "Current Value ($)".
- **Pie Charts:** Font size for labels reduced for better readability.
- **Table Font Size:** All table data cells use font size 10 for compactness.
- **Pagination:** Per-asset table paginates filtered results, with page selector at the bottom.
- **Immediate Filtering:** Table updates instantly when any filter changes.

---

## Troubleshooting

- **CORS errors:**  
  The backend is configured to allow requests from any origin. If you change ports or deploy, adjust the CORS settings in `main.rs`.

- **API key errors:**  
  Make sure your `.env` file is present and contains a valid CoinMarketCap API key and the `MARKET` variable.

- **Dependency issues:**  
  Run `cargo update` in the backend and `npm install` in the frontend if you encounter build errors.

---

## Customization

- The frontend uses Material UI with a dark theme.  
  You can further customize the look in `frontend/src/App.jsx`.
- The backend reads your portfolio from `wallet_allocations.csv`.  
  You can automate or extend this as needed.

---

## History API & Dashboard Data

Every time `/api/allocations` runs (e.g., when you click “Update Prices & Show Distribution”) the backend now persists the computed snapshot directly into SQLite:

- `history_assets` receives one row per asset (with target %, current %, deviation %, and USD value deviation computed in the database).
- `history_groups` stores the per-group view that powers both the table and the dashboard.
- `history_barca` and `history_totals` keep BARCA-level and total wallet timelines.
- The derived views `asset_variance_history`, `group_variance_history`, and `barca_variance_history` are what `/api/history` serves to the frontend.

API:
- `GET /api/allocations` — computes the latest allocation, persists the snapshot, and returns the live tables/charts.
- `GET /api/history?level={totals|assets|barca|groups}` — streams the historical rows for the requested level. Assets and BARCA entries now include `deviation` and `value_deviation` fields for the variance dashboard.

Example:

```bash
curl -sS "http://127.0.0.1:3001/api/history?level=assets" | jq .
```

The frontend dashboard tab uses these APIs (with optional period bucketing) to plot totals, BARCA groups, asset symbols, or the new per-group series.

## Migrating from CSV to DB

Current behavior remains CSV-first: the app still writes CSV snapshots. The DB migration and import tool let you keep the CSV as the editable source-of-truth for wallet definitions while persisting snapshots and allocation audits in SQLite for queries, dashboards, and tests.

Planned next steps (optional):

- Wire `/api/allocations` to persist snapshots into the DB (instead of CSVs).
- Add automated migration runner using `sqlx::migrate!` and CI checks.



## License

MIT

---
