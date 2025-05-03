# Crypto Management Application

This project is a full-stack cryptocurrency portfolio management tool.
It features a **Rust (Axum) backend** for live price and allocation calculations, and a **React (Material UI) frontend** for a modern, interactive dashboard.

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

   Create a `.env` file in the project root with your CoinMarketCap API key:

   ```
   API_KEY=your_coinmarketcap_api_key
   ```

3. **Prepare your wallet allocations file:**

   Edit or create `wallet_allocations.csv` in the project root. Example:

   ```
   symbol,group,target_percent,current_quantity
   USDT,Caixa,30,1200
   BTC,Holding,10,0.75
   ETH,Holding,10,2.5
   SOL,Trading,5,10
   ETH,Trading,15,2
   DOGE,Trading,2,1000
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

- Click **"Update Prices & Show Distribution"** in the frontend to fetch live prices and see your portfolio allocation.
- The dashboard displays both per-asset and per-group allocation, with deviations highlighted.
- To update your portfolio, edit `wallet_allocations.csv` and refresh the frontend.

---

## Troubleshooting

- **CORS errors:**  
  The backend is configured to allow requests from any origin. If you change ports or deploy, adjust the CORS settings in `main.rs`.

- **API key errors:**  
  Make sure your `.env` file is present and contains a valid CoinMarketCap API key.

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