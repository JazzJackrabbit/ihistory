# ihistory - Bash integration
# Usage: eval "$(ihistory --init bash)"

# Load ihistory saved aliases from config file
_ih_load_aliases() {
  local file
  case "$(uname -s)" in
    Darwin) file="$HOME/Library/Application Support/ihistory/aliases" ;;
    *)      file="${XDG_CONFIG_HOME:-$HOME/.config}/ihistory/aliases" ;;
  esac
  [[ -f "$file" ]] || return 0
  local line name cmd
  while IFS= read -r line || [[ -n "$line" ]]; do
    name="${line%%=*}"
    cmd="${line#*=}"
    [[ -z "$name" || "$name" = "$line" ]] && continue
    alias "$name"="$cmd"
  done < <(tr '\0' ' ' < "$file")
}

# ih function - interactive history search
ih() {
  local selected ret
  selected="$(ihistory "$@")"
  ret=$?
  # Reload aliases in case they were modified
  _ih_load_aliases
  if [[ -n "$selected" ]]; then
    if [[ $ret -eq 10 ]]; then
      eval "$selected"
    else
      history -s "$selected"
      echo "$selected"
    fi
  fi
}

ih-widget() {
  local selected
  selected="$(ihistory)"
  local ret=$?

  if [[ $ret -eq 0 && -n "$selected" ]]; then
    READLINE_LINE="$selected"
    READLINE_POINT=${#selected}
  elif [[ $ret -eq 10 && -n "$selected" ]]; then
    READLINE_LINE="$selected"
    READLINE_POINT=${#selected}
  fi
}

# Load ihistory aliases on init
_ih_load_aliases
