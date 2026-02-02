#!/bin/bash

HIST_FILE="$HOME/.zsh_history"
BACKUP_FILE="$HOME/.zsh_history.backup"

case "$1" in
  backup)
    cp "$HIST_FILE" "$BACKUP_FILE"
    echo "Backed up to $BACKUP_FILE"
    ;;

  populate)
    cat >> "$HIST_FILE" << 'EOF'
: 1767312000:0;git status
: 1767312060:0;git commit -m "fix login bug"
: 1767312120:0;git push origin main
: 1767312180:0;git checkout -b feature/auth
: 1767312240:0;git log --oneline -10
: 1767312300:0;docker compose up -d
: 1767312360:0;docker logs -f api
: 1767312420:0;docker ps
: 1767312480:0;npm install
: 1767312540:0;npm run dev
: 1767312600:0;npm run build
: 1767312660:0;npm test
: 1767312720:0;cargo build --release
: 1767312780:0;cargo test
: 1767312840:0;python manage.py migrate
: 1767312900:0;curl -X POST https://api.example.com/users
: 1767312960:0;kubectl get pods
: 1767313020:0;vim config.yaml
: 1767313080:0;brew update && brew upgrade
EOF
    echo "Added sample commands to history"
    ;;

  restore)
    if [ -f "$BACKUP_FILE" ]; then
      cp "$BACKUP_FILE" "$HIST_FILE"
      echo "Restored from backup"
    else
      echo "No backup found"
    fi
    ;;

  clear)
    > "$HIST_FILE"
    echo "Cleared history"
    ;;

  *)
    echo "Usage: $0 {backup|populate|clear|restore}"
    ;;
esac
