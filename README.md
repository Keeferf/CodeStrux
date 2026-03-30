<div align="center">
  <img src="./public/icon.svg" alt="CodeStrux logo" width="96" />
  <h1>CodeStrux</h1>
  <p>A private, fully offline AI chat app for your desktop. Download models. Run them locally. No cloud, no accounts, no data leaving your machine.</p>

  <p>
    <img src="https://img.shields.io/badge/Tauri-2.x-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri" />
    <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=white" alt="React" />
    <img src="https://img.shields.io/badge/Rust-stable-CE422B?style=flat-square&logo=rust&logoColor=white" alt="Rust" />
    <img src="https://img.shields.io/badge/TypeScript-strict-3178C6?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript" />
    <img src="https://img.shields.io/badge/license-MIT-ffba49?style=flat-square" alt="License" />
  </p>

  <a href="https://github.com/Keeferf/CodeStrux/releases/latest">
    <img src="https://img.shields.io/badge/⬇ Download-Windows Installer-20a39e?style=for-the-badge" alt="Download" />
  </a>
</div>

---

## Installation

> **For users who just want to run the app — no coding required.**

1. Go to the [**Releases**](https://github.com/Keeferf/CodeStrux/releases/latest) page
2. Under **Assets**, download the installer for your platform
3. Run the installer
4. Launch **CodeStrux** from your Start menu or desktop

That's it. No Node.js, no Rust, no terminal needed.

---

## Overview

CodeStrux is a native desktop AI chat application built with Tauri and React. It downloads open-weight language models directly from HuggingFace and runs them locally using a bundled `llama-server` binary — no internet connection is required after the initial model download. Chat sessions, model state, and settings are all stored on your machine.

---

## Features

- **100% local inference** — models run via a bundled `llama-server` subprocess; nothing is sent to a cloud API
- **Model downloader** — fetch GGUF models from HuggingFace with a real-time progress bar and cancel support
- **Multiple sessions** — create, switch between, and delete chat sessions from the collapsible sidebar
- **Creativity modes** — choose between Precise, Balanced, and Creative temperature presets
- **Hardware detection** — reports your GPU/CPU info to help with model selection
- **Settings panel** — manage downloaded models, load/unload the active model, and tune inference settings
- **Streaming responses** — tokens stream into the chat window in real time via Tauri events
- **Fully offline** — once a model is downloaded, no network access is required

---

## Tech Stack

| Layer         | Technology                                                                                                  |
| ------------- | ----------------------------------------------------------------------------------------------------------- |
| Desktop shell | [Tauri 2](https://tauri.app)                                                                                |
| Frontend      | [React 19](https://react.dev) + [Vite](https://vitejs.dev)                                                  |
| Language      | [TypeScript](https://www.typescriptlang.org/) (strict)                                                      |
| Styling       | [Tailwind CSS v4](https://tailwindcss.com)                                                                  |
| Persistence   | [`tauri-plugin-store`](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/store) (JSON, local) |
| Inference     | [`llama-server`](https://github.com/ggml-org/llama.cpp) (bundled binary)                                    |
| HTTP client   | [reqwest](https://github.com/seanmonstar/reqwest) (rustls, HTTP/1.1 chunked downloads)                      |
| Icons         | [Lucide React](https://lucide.dev)                                                                          |

---

## Prerequisites

Before you begin, make sure you have the following installed:

- [Node.js](https://nodejs.org) 18+
- [Rust](https://rustup.rs) stable toolchain (`rustup update stable`)
- Platform-specific Tauri dependencies — follow the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for your OS

---

## Project Structure

```
codestrux/
├── src/                              # React + TypeScript frontend
│   ├── components/
│   │   ├── chat/                     # ChatArea — message list, input bar, streaming
│   │   ├── hardware/                 # Hardware info display
│   │   ├── layout/                   # Header + Sidebar (session list, nav)
│   │   ├── settings/                 # SettingsPanel — model management, creativity
│   │   └── ui/                       # Shared primitive components
│   ├── constants/
│   │   └── creativity.ts             # Precise / Balanced / Creative temp presets
│   ├── lib/
│   │   └── Download.ts               # Download progress events + cancel helper
│   ├── types/
│   │   └── index.ts                  # ChatMessage, Session, CreativityKey types
│   ├── App.tsx                       # Root — session state, chat handlers, event wiring
│   └── index.css                     # Tailwind v4 theme tokens
│
└── src-tauri/                        # Rust / Tauri backend
    ├── src/
    │   ├── main.rs
    │   ├── lib.rs                    # Tauri builder, plugin setup, command registration
    │   ├── model_storage.rs          # Downloaded model registry (list + delete)
    │   ├── hardware.rs               # GPU/CPU detection via sysinfo + wgpu
    │   ├── chat/
    │   │   ├── mod.rs
    │   │   ├── commands.rs           # get/load/unload model; start/stop chat
    │   │   ├── server.rs             # llama-server subprocess lifecycle
    │   │   ├── state.rs              # LocalChatState (Arc<Mutex<…>>)
    │   │   ├── types.rs              # Rust chat message types + serde
    │   │   └── logging.rs            # llama-server stdout/stderr relay
    │   └── download/
    │       ├── mod.rs
    │       ├── commands.rs           # start_download / cancel_download Tauri commands
    │       ├── client.rs             # reqwest HTTP/1.1 chunked downloader
    │       ├── transfer.rs           # Per-chunk mpsc channel → disk writer
    │       ├── validate.rs           # File integrity checks
    │       └── types.rs              # DownloadProgress + DownloadState
    └── bin/                          # Bundled llama-server binaries (platform-specific)
```

---

## Tauri Commands

The frontend communicates with the Rust backend via Tauri's `invoke` API. All commands are async and return `Result<T, String>`.

| Command                   | Arguments                     | Returns                   |
| ------------------------- | ----------------------------- | ------------------------- |
| `get_hardware_info`       | —                             | `HardwareInfo`            |
| `get_downloaded_models`   | —                             | `DownloadedModel[]`       |
| `delete_downloaded_model` | `{ modelId, filename }`       | `void`                    |
| `start_download`          | `{ modelId, filename, url }`  | `void`                    |
| `cancel_download`         | —                             | `void`                    |
| `get_loaded_model`        | —                             | `LoadedModelInfo \| null` |
| `load_local_model`        | `{ modelId, filename }`       | `void`                    |
| `unload_local_model`      | —                             | `void`                    |
| `start_local_chat`        | `{ messages: ChatMessage[] }` | `void`                    |
| `stop_local_chat`         | —                             | `void`                    |

### Tauri Events

Streaming responses and download progress are pushed from Rust to the frontend via events:

| Event                | Payload            | Description                           |
| -------------------- | ------------------ | ------------------------------------- |
| `local-chat-token`   | `string`           | Next streamed token from llama-server |
| `local-chat-done`    | —                  | Inference completed                   |
| `local-chat-error`   | `string`           | Inference error message               |
| `model-loaded`       | `LoadedModelInfo`  | Model finished loading                |
| `model-error`        | —                  | Model failed to load                  |
| `unload-model`       | —                  | Model unloaded                        |
| `download-progress`  | `DownloadProgress` | Bytes received / total                |
| `download-done`      | —                  | Download completed successfully       |
| `download-cancelled` | —                  | Download was cancelled                |
| `download-error`     | `string`           | Download error message                |

---

## Creativity Modes

Creativity is controlled by adjusting the temperature passed to `llama-server`:

| Mode     | Temperature | Description                      |
| -------- | ----------- | -------------------------------- |
| Precise  | `0.2`       | Deterministic, factual responses |
| Balanced | `0.7`       | Default general-purpose setting  |
| Creative | `1.2`       | Expressive, varied responses     |

---
