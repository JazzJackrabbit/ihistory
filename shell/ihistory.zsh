# ihistory — zsh integration
# Usage: eval "$(ihistory --init zsh)"

# An existing `ih` alias would shadow the function definition below.
unalias ih 2>/dev/null

ih() {
  emulate -L zsh
  local selected ret
  selected="$(command ihistory "$@")"
  ret=$?
  [[ -n "$selected" ]] || return 0
  if (( ret == 10 )); then
    # Record before running: an eval'd command never reaches history on its own.
    print -s -- "$selected"
    eval "$selected"
  else
    print -z -- "$selected"
  fi
}

ih-widget() {
  emulate -L zsh
  local selected ret saved_buffer="$BUFFER" saved_cursor="$CURSOR"
  selected="$(command ihistory)"
  ret=$?
  zle reset-prompt
  if [[ -n "$selected" ]] && (( ret == 10 )); then
    BUFFER="$selected"
    zle accept-line
    return
  elif [[ -n "$selected" ]] && (( ret == 0 )); then
    BUFFER="$selected"
    CURSOR=${#BUFFER}
  else
    BUFFER="$saved_buffer"
    CURSOR="$saved_cursor"
  fi
  zle redisplay
}
zle -N ih-widget

# Ctrl+R opens the search in every keymap. Export IHISTORY_NO_BINDKEY=1
# before the eval line to keep your existing binding.
if [[ -z "$IHISTORY_NO_BINDKEY" ]]; then
  bindkey -M emacs '^R' ih-widget
  bindkey -M viins '^R' ih-widget
  bindkey -M vicmd '^R' ih-widget
fi
