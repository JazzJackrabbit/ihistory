# ihistory — bash integration
# Usage: eval "$(ihistory --init bash)"

ih() {
  local selected ret
  selected="$(command ihistory "$@")"
  ret=$?
  [[ -n "$selected" ]] || return 0
  # Record either way: an eval'd command never reaches history on its own.
  history -s "$selected"
  if (( ret == 10 )); then
    eval "$selected"
  else
    printf '%s\n' "$selected"
  fi
}

# Readline integration is only meaningful (and only safe to bind) in an
# interactive shell.
if [[ $- == *i* ]]; then
  ih-widget() {
    local selected ret
    selected="$(command ihistory)"
    ret=$?
    # bind -x cannot accept-line, so run-immediately degrades to inserting
    # the command for a confirming Enter.
    if [[ -n "$selected" ]] && { (( ret == 0 )) || (( ret == 10 )); }; then
      READLINE_LINE="$selected"
      READLINE_POINT=${#READLINE_LINE}
    fi
  }

  # Ctrl+R opens the search. Export IHISTORY_NO_BINDKEY=1 before the eval
  # line to keep your existing binding.
  if [[ -z "$IHISTORY_NO_BINDKEY" ]]; then
    bind -x '"\C-r": ih-widget' 2>/dev/null
  fi
fi
