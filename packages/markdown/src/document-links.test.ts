import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { followDocumentLinks } from "./document-links";

/**
 * A viewer holding one document link as the renderer writes it, minus the
 * `onmousedown` attribute: a page whose Content-Security-Policy blocks inline
 * handlers never runs it, which is the case this module is for.
 */
function viewerWithLink(): { viewer: HTMLElement; link: HTMLElement } {
  const viewer = document.createElement("div");
  viewer.innerHTML =
    '<article class="markdown-body"><p><span data-md-link="./other.md#part" class="md-link"><em>there</em></span></p></article>';
  document.body.append(viewer);
  return { viewer, link: viewer.querySelector<HTMLElement>(".md-link")! };
}

function press(target: Element, button: number): MouseEvent {
  const event = new MouseEvent("mousedown", { button, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

let clicked: ReturnType<typeof vi.fn>;

beforeEach(() => {
  clicked = vi.fn();
  vi.stubGlobal("handleMarkdownLinkClick", clicked);
});

afterEach(() => {
  document.body.replaceChildren();
  vi.unstubAllGlobals();
});

describe("followDocumentLinks", () => {
  test("hands a pressed link's target to the host, as the document wrote it", () => {
    const { viewer, link } = viewerWithLink();
    followDocumentLinks(viewer);

    const event = press(link.querySelector("em")!, 0);

    expect(clicked).toHaveBeenCalledExactlyOnceWith("./other.md#part", 0);
    expect(event.defaultPrevented).toBe(true);
  });

  test("hands over a middle-button press too", () => {
    const { viewer, link } = viewerWithLink();
    followDocumentLinks(viewer);

    press(link, 1);

    expect(clicked).toHaveBeenCalledExactlyOnceWith("./other.md#part", 1);
  });

  test("leaves a right-button press to the page", () => {
    const { viewer, link } = viewerWithLink();
    followDocumentLinks(viewer);

    const event = press(link, 2);

    expect(clicked).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(false);
  });

  test("does nothing when the link's own handler already ran", () => {
    // Where inline handlers are allowed, the `onmousedown` the renderer
    // writes runs first, on the link, and prevents the default; the host
    // must not be told twice.
    const { viewer, link } = viewerWithLink();
    link.addEventListener("mousedown", (event) => event.preventDefault());
    followDocumentLinks(viewer);

    press(link, 0);

    expect(clicked).not.toHaveBeenCalled();
  });

  test("listens once however often the viewer is rendered", () => {
    const { viewer, link } = viewerWithLink();
    followDocumentLinks(viewer);
    followDocumentLinks(viewer);

    press(link, 0);

    expect(clicked).toHaveBeenCalledOnce();
  });

  test("ignores a press on anything but a document link", () => {
    const { viewer } = viewerWithLink();
    followDocumentLinks(viewer);

    press(viewer.querySelector("p")!, 0);

    expect(clicked).not.toHaveBeenCalled();
  });
});
