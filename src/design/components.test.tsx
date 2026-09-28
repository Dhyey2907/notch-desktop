import { act, fireEvent, render, screen } from "@testing-library/react";
import { Play } from "lucide-react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ConfirmButton, Heatmap, IconButton, MediaControl, Progress } from "./components";
import { heatmapLevel } from "./heatmap";

afterEach(() => vi.useRealTimers());

describe("ConfirmButton", () => {
  it("requires a second click to confirm", () => {
    const onConfirm = vi.fn();
    render(
      <ConfirmButton confirmLabel="Confirm push?" onConfirm={onConfirm}>
        Push
      </ConfirmButton>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Push" }));
    expect(onConfirm).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Confirm push?" }));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Push" })).toBeInTheDocument();
  });

  it("disarms after the timeout", () => {
    vi.useFakeTimers();
    const onConfirm = vi.fn();
    render(
      <ConfirmButton confirmLabel="Sure?" onConfirm={onConfirm} timeoutMs={1000}>
        Delete
      </ConfirmButton>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    act(() => vi.advanceTimersByTime(1001));
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(onConfirm).not.toHaveBeenCalled();
  });
});

describe("accessibility", () => {
  it("gives icon buttons an accessible name", () => {
    render(<IconButton icon={Play} label="Play" />);
    expect(screen.getByRole("button", { name: "Play" })).toBeInTheDocument();
  });

  it("disables unsupported media controls instead of hiding them", () => {
    render(<MediaControl playing={false} canNext={false} />);
    expect(screen.getByRole("button", { name: "Next track" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Play" })).toBeEnabled();
  });

  it("labels every heatmap cell with its value, not just a color", () => {
    render(
      <Heatmap
        label="Commits"
        unit="commits"
        cells={[
          { date: "2026-09-01", value: 0, level: 0 },
          { date: "2026-09-02", value: 12, level: 4 },
        ]}
      />,
    );
    expect(screen.getByLabelText("2026-09-02: 12 commits")).toBeInTheDocument();
  });

  it("exposes progress as a progressbar", () => {
    render(<Progress label="Upload" value={3} max={4} />);
    expect(screen.getByRole("progressbar", { name: "Upload" })).toHaveAttribute("aria-valuenow", "3");
  });
});

describe("heatmapLevel", () => {
  it("buckets relative to the busiest day", () => {
    expect(heatmapLevel(0, 10)).toBe(0);
    expect(heatmapLevel(1, 10)).toBe(1);
    expect(heatmapLevel(4, 10)).toBe(2);
    expect(heatmapLevel(7, 10)).toBe(3);
    expect(heatmapLevel(10, 10)).toBe(4);
    expect(heatmapLevel(5, 0)).toBe(0);
  });
});
