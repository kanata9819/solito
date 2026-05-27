# Solito Terminal Emulator

Solito is a small terminal emulator built with Rust.

It uses `winit` for the native window, `wgpu` for rendering, `glyphon` for text,
and `portable-pty` for shell sessions. The default shell is `nu`.

## Features

- GPU-rendered terminal text
- PTY-backed shell sessions
- Multiple tabs with terminal-style shortcuts
- Keyboard copy mode
- Configurable font and window backdrop

## Concept

コーディングエージェントによるコード生成を行わず作成するコンセプトのターミナルエミュレータ。

**ルール**

1. 生成AI / AIエージェント が生成したコードを使用するのは禁止
2. 生成AIへの質問はOK
3. AIエージェントを使用できるのは、コードレビュー / コードベースの質問の場合のみ
4. Google検索OK

since 2026/04/15
