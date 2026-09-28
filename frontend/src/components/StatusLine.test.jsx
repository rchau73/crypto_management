import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusLine } from "./StatusLine";

describe("StatusLine", () => {
  it("renders nothing when there's no update, status, or error", () => {
    const { container } = render(<StatusLine lastUpdate={null} importStatus="" importError="" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the import status text on success", () => {
    render(<StatusLine lastUpdate={null} importStatus="Imported 3 wallet rows from CSV" importError="" />);
    expect(screen.getByText("Imported 3 wallet rows from CSV")).toBeInTheDocument();
  });

  it("shows the import error inline, in its own paragraph separate from the status line", () => {
    render(<StatusLine lastUpdate={null} importStatus="" importError="Invalid CSV file: missing field `symbol`" />);
    const error = screen.getByText("Invalid CSV file: missing field `symbol`");
    expect(error.tagName).toBe("P");
  });

  it("shows both the last-update status and the error together when a later import fails", () => {
    render(
      <StatusLine
        lastUpdate={new Date("2026-01-01T00:00:00Z")}
        importStatus=""
        importError="Invalid CSV file: missing field `symbol`"
      />
    );
    expect(screen.getByText(/Last update:/)).toBeInTheDocument();
    expect(screen.getByText("Invalid CSV file: missing field `symbol`")).toBeInTheDocument();
  });
});
