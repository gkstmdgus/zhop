# vhs tape for the zhop demo GIF.  Render:  vhs demo/demo.tape
# Produces assets/demo.gif

Output assets/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 700
Set Padding 16
Set Theme "Catppuccin Mocha"

# --- hidden setup: drop any inherited Zellij session, clean slate ---
Hide
Type "unset ZELLIJ ZELLIJ_SESSION_NAME ZELLIJ_PANE_ID; clear"
Enter
Sleep 600ms
Show

# launch the demo session (-n forces a brand-new session with this layout)
Type "zellij -s zhop-demo --config demo/config.kdl -n demo/layout.kdl"
Enter
Sleep 3500ms

# open zhop (only zhop shows, default floating size)
Ctrl+y
Sleep 2s

# NORMAL mode: j/k fly around with no typing
Type "j"
Sleep 500ms
Type "j"
Sleep 500ms
Type "j"
Sleep 600ms
Type "k"
Sleep 800ms

# / to filter by name
Type "/"
Sleep 500ms
Type "lo"
Sleep 1s

# enter switches to the highlighted tab (logs)
Enter
Sleep 1800ms

# do it again: jump straight to "git"
Ctrl+y
Sleep 1200ms
Type "/"
Sleep 300ms
Type "git"
Sleep 900ms
Enter
Sleep 2s
