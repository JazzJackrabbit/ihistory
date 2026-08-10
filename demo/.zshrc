PS1="$ "

# Pin the dev build explicitly: PATH order alone is not enough, because the
# shell may have already hashed `ihistory` to an installed copy.
dev_binary="${0:A:h}/../target/release/ihistory"
if [[ -x "$dev_binary" ]]; then
  path=("${dev_binary:h}" $path)
  hash ihistory="$dev_binary"
fi

source "${0:A:h}/../shell/ihistory.zsh"
