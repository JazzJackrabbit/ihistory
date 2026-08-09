# VHS demo script
# Run: ./setup.sh backup && ./setup.sh clear && ./setup.sh populate && vhs demo.tape && ./setup.sh restore
#
# The recording launches via `ih` — vhs's pty cannot host the Ctrl+R zle
# widget (its subshell breaks the TUI's input reader), real terminals can.

Output demo.gif

Set FontSize 18
Set Width 800
Set Height 450
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 100ms
Set Shell "zsh"

Hide
Type "source .zshrc"
Enter
Type "clear"
Enter
Show

Sleep 1s

Type "ih"
Sleep 500ms
Enter
Sleep 1.5s

Type "git"
Sleep 1.5s

Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 1s

Enter
Sleep 2.5s
