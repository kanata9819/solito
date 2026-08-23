set windows-shell := ["cmd.exe", "/C"]

# Measure Solito with a real automated Neovim session.
bench seconds="10":
    cargo build --release -p solito -p solito-bench
    target\release\solito-bench.exe solito --mode nvim --seconds {{seconds}}

# Measure Solito's deterministic full-screen ANSI repaint performance.
bench-ansi seconds="10":
    cargo build --release -p solito -p solito-bench
    target\release\solito-bench.exe solito --mode full --seconds {{seconds}}
