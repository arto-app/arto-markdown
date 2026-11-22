/**
 * Document links, for a page whose Content-Security-Policy blocks inline
 * event handlers.
 *
 * The renderer writes each document link with an `onmousedown` attribute
 * that calls `window.handleMarkdownLinkClick`, and Arto's own windows run it.
 * A page that allows scripts only from files or by hash never runs an inline
 * handler, so its links would do nothing. The viewer therefore listens for
 * the press itself. Where the inline handler did run, it has already
 * prevented the default, and the host is not told a second time.
 */

declare global {
  interface Window {
    /** The host's: open `path`, as the document wrote it, for `button`. */
    handleMarkdownLinkClick?: (path: string, button: number) => void;
  }
}

const LINK = ".md-link";

const following = new WeakSet<Element>();

/** Hand a left or middle press on a document link in `viewer` to the host. */
export function followDocumentLinks(viewer: Element): void {
  if (following.has(viewer)) {
    return;
  }
  following.add(viewer);
  viewer.addEventListener("mousedown", (event) => {
    if (!(event instanceof MouseEvent) || event.defaultPrevented) {
      return;
    }
    if (event.button !== 0 && event.button !== 1) {
      return;
    }
    const link = event.target instanceof Element ? event.target.closest(LINK) : null;
    if (!(link instanceof HTMLElement) || !viewer.contains(link)) {
      return;
    }
    const path = link.dataset.mdLink;
    if (path === undefined) {
      return;
    }
    event.preventDefault();
    window.handleMarkdownLinkClick?.(path, event.button);
  });
}
