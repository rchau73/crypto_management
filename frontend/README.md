# Frontend

React 19 + MUI + Recharts single-page app for the Wallet Allocations dashboard. For project-wide setup (backend, database, running both halves together), see the [root README](../README.md#frontend-react-setup).

```bash
npm install
npm run dev      # start the dev server
npm run test     # run the Vitest suite
npm run lint     # eslint
npm run build    # production build
```

Copy `.env.example` to `.env.local` if the backend isn't running on the default `http://localhost:3001`.

---

## Architecture

`App.jsx` is a thin composition root — it owns the tab/filter state and derived totals, and delegates everything else. No business logic lives in a component: table math is in `utils/`, data fetching is in `hooks/`, and styling tokens are in `theme.js`.

Source: [`docs/architecture.mmd`](docs/architecture.mmd)

![Frontend architecture: App.jsx composition root wiring layout components, one component per tab, shared UI components, data-fetching hooks, pure utils, the API client, and the theme](docs/architecture.png)

### Key components structure

| Layer | Path | Responsibility |
|---|---|---|
| Composition root | `src/App.jsx` | Tab/filter state, derived totals (`filteredAllocations`, `symbolAllocations`), and wiring everything else together. Nothing here renders a table row directly. |
| Layout | `src/components/AppHeader.jsx`, `ActionsBar.jsx`, `StatTiles.jsx`, `StatusLine.jsx`, `FiltersBar.jsx`, `EmptyState.jsx` | The page chrome: sticky header, KPI tiles, filter chips, empty/loading affordances. |
| Tabs | `src/components/tabs/*.jsx` | One component per tab (`PerAssetTab`, `PerAssetTotalTab`, `PerGroupTab`, `BarcaActualTab`, `DashboardTab`). Each owns its own table/chart composition and pulls in whichever hooks/utils it needs. |
| Shared UI | `src/components/{SortableTableHead,PaginationControls,DeviationBadge,AllocationPieChart,HistoryLineChart,ChartTooltip,DashboardControls}.jsx` | Reusable pieces used by more than one tab — a generic sortable header, pagination controls, the colored-dot deviation indicator, and the two chart wrappers. |
| Hooks | `src/hooks/{useAllocations,useHistoryDashboard,useSortableData,usePagination}.js` | All `useState`/`useEffect`/data-fetching. `useSortableData` and `usePagination` are generic — every table reuses the same two hooks instead of hand-rolling sort/paging logic per table. |
| Utils | `src/utils/{formatters,allocationMath,historyBucketing}.js` | Pure functions only — no React, no DOM. This is 100% of what's unit-tested (`*.test.js` next to each file). |
| API | `src/api/client.js` | The only place that calls `fetch()`. `VITE_API_BASE_URL` (see `.env.example`) points it at the backend. |
| Theme | `src/theme.js` | The dark MUI theme, the fixed 8-color categorical palette, and `colorForSeries()` — see [Design system](#design-system) below. |

### Testing

Vitest + React Testing Library. Pure logic in `utils/` and the two generic hooks have direct unit tests; `PaginationControls` has a component test exercising click behavior. Run with `npm run test`. There's no end-to-end browser test — the golden path and edge cases (filters, sorting, pagination, CSV import, all 5 tabs, both chart types) are verified manually against a running backend before every change of this size.

---

## Design system

The UI follows a validated dark-theme palette rather than MUI's stock `mode: "dark"` defaults — see `theme.js` for the full token set.

- **Categorical color is fixed-order, not cycled by array index.** `colorForSeries(name, allSeries)` resolves a color from an entity's position in the *full* sorted series list, not its position in whatever subset is currently selected/filtered — so removing a filter never repaints the series that survive it.
- **Status colors (`good`/`warning`/`critical`) are reserved** for deviation severity (`DeviationBadge`) and never reused as a chart series color, so a red line and a "critical deviation" cell never look like the same signal.
- **Known trade-off**: the fixed palette has 8 hues. With ~30 possible assets, an arbitrary 5-series default (see below) can occasionally repeat a color. This is an accepted limit of a fixed categorical palette, not a bug — narrow the Dashboard's Series picker to avoid it when it matters.
- **The Dashboard's "Assets"/"BARCA"/"Groups" levels default to the top 5 series by value**, not every series — a line chart with 30 overlapping series is unreadable regardless of color choice; the multi-select still allows picking any subset.
- Numeric table columns use `font-variant-numeric: tabular-nums` so digits align vertically down a column.
- A deviation reading is never color-only: `DeviationBadge` always pairs the color with a visible dot and the text itself, for colorblind-safe reading.
