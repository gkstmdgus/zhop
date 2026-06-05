# vhs tape for the zhop demo GIF.  Render:  vhs demo/demo.tape
# Produces assets/demo.gif

Output assets/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 1120
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

# open zhop — tabs are grouped by their category prefix (dev / ops / web)
Ctrl+y
Sleep 2200ms

# Tab toggles to a flat list…
Tab
Sleep 1600ms
# …and back to the grouped view
Tab
Sleep 1600ms

# NORMAL mode: j/k fly around with no typing
Type "j"
Sleep 400ms
Type "j"
Sleep 400ms
Type "j"
Sleep 700ms

# / to filter by name — narrows to ops:metrics, Enter jumps to it
Type "/"
Sleep 400ms
Type "metr"
Sleep 1100ms
Enter
Sleep 1800ms

# reopen and jump straight by index number: type 1 then 0 → tab #10, Enter
Ctrl+y
Sleep 1500ms
Type "1"
Sleep 700ms
Type "0"
Sleep 900ms
Enter
Sleep 2000ms
