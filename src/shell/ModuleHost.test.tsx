import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ModuleErrorBoundary } from "./ModuleHost";

describe("ModuleErrorBoundary", () => {
  it("contains a crashing module and lets it be reloaded", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    let shouldThrow = true;
    function Flaky() {
      if (shouldThrow) throw new Error("boom");
      return <p>Recovered</p>;
    }

    render(
      <>
        <ModuleErrorBoundary name="Media">
          <Flaky />
        </ModuleErrorBoundary>
        <p>Shell still here</p>
      </>,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("Media hit a problem");
    expect(screen.getByText("Shell still here")).toBeInTheDocument();
    expect(screen.queryByText(/boom/)).not.toBeInTheDocument(); // no raw error text in the UI

    shouldThrow = false;
    fireEvent.click(screen.getByRole("button", { name: "Reload" }));
    expect(screen.getByText("Recovered")).toBeInTheDocument();
    spy.mockRestore();
  });
});
