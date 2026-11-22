# Concept

This document describes what arto-markdown is for and the ideas every design
decision rests on. It is rewritten in place as the concept evolves; decisions
built on it are recorded as ADRs.

## What it is

The engine that draws a Markdown document the way Arto, the reader, draws it:
a renderer that turns Markdown into HTML, and a frontend that draws in a page
what that HTML leaves to script — diagrams, formulas, HTML embeds and the
colours of code. It is released on its own so that any host can show a
document as Arto does without being Arto.

## Who it is for

- Arto itself, in its window, in `arto page` and in Quick Look.
- Applications that show Markdown in a window or a web page of their own and
  want GitHub's reading of it, with the extensions Arto reads.
- Hosts that are not a desktop with a disk: a browser, a phone, a server that
  keeps its documents in a database.

## Principles

- **The HTML is a contract.** The attributes and class names the renderer
  writes are documented, and the frontend reads nothing else. The sample
  documents pin the output, so a change to it is a reviewed diff, never a
  surprise.
- **GitHub is the baseline.** A document written for GitHub renders as it
  does there: tables, task lists, footnotes and alerts are not options. What
  goes beyond GitHub is added without changing what GitHub already means.
- **The host keeps its page.** The stylesheet styles the viewer element and
  nothing outside it; the runtime draws when it is asked to and takes no
  keys, scrolling or theme of its own.
- **The host answers for its files.** Whether a linked document exists and
  what an image holds are questions put to the host; reading this machine's
  disk is one answer among others.
- **One version for both halves.** The renderer and the frontend are built
  from the same commit and released together, so the markup the frontend
  reads is the markup the renderer writes.

## What it does not do

- Edit documents. It reads Markdown and writes HTML.
- Fetch anything on its own: what a document names elsewhere is the host's
  to load or refuse.
- Own a window or an application: keybindings, navigation, settings and
  file watching belong to the host.
