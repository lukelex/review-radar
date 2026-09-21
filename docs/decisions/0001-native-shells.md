# ADR 0001: Use native platform shells with a Rust core

## Status

Accepted.

## Context

Review Radar must support Linux, macOS, and Windows while feeling native on each
platform. The product is a compact, frequently accessed desktop workspace with
important platform interactions: tray/menu-bar entry points, notifications,
shortcuts, accessibility, and browser launching.

## Decision

Use a shared Rust core for product logic and native platform shells:

- Qt/QML on Linux;
- SwiftUI on macOS;
- WinUI 3 on Windows.

Quickshell is a Linux integration adapter, not the primary product runtime.

## Consequences

- Platform shells require separate UI work.
- GitHub querying, priority logic, and local state stay shared and testable.
- The application can follow each operating system's visual and interaction
  conventions instead of presenting a web or custom-rendered port everywhere.
