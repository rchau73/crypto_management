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
   - Reads wallet allocations from `wallet_allocations.csv`.
   - Reads BARCA targets from `wallet_barca.csv`.
   - Serves `/api/allocations` with all computed data.

2. **Run the backend:**
   ```sh
   cargo run
   ```

- Click **"Update Prices & Show Distribution"** in the frontend to fetch live prices and see your portfolio allocation.
- The dashboard displays both per-asset and per-group allocation, with deviations highlighted.
- To update your portfolio, edit `wallet_allocations.csv` and refresh the frontend.

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

## License

MIT

---