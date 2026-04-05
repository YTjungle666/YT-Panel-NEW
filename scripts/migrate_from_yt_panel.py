#!/usr/bin/env python3
"""Migrate runtime data from the legacy YT-Panel service into YT-panel-Rust.

This script is intentionally conservative:
- it copies the legacy SQLite DB instead of mutating it in place
- it copies uploads instead of moving them
- it refuses to overwrite an existing Rust runtime unless --force is given

Default paths are tailored to the current OpenClaw workspace layout.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sqlite3
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Iterable


SCRIPT_PATH = Path(__file__).resolve()
REPO_ROOT = SCRIPT_PATH.parent.parent
WORKSPACE_ROOT = REPO_ROOT.parent.parent
DEFAULT_OLD_SERVICE_ROOT = WORKSPACE_ROOT / "projects" / "YT-Panel" / "service"
DEFAULT_NEW_RUNTIME_ROOT = WORKSPACE_ROOT / "artifacts" / "YT-panel-Rust" / "runtime"

NOTICE_TABLE_SQL = """
CREATE TABLE IF NOT EXISTS notice (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    title TEXT,
    content TEXT,
    display_type INTEGER,
    one_read INTEGER DEFAULT 0,
    url TEXT,
    is_login INTEGER DEFAULT 0,
    user_id INTEGER
)
""".strip()

DEFAULT_SETTINGS = {
    "system_application": json.dumps({"loginCaptcha": False, "register": False}, separators=(",", ":")),
    "disclaimer": "",
    "web_about_description": "",
}


@dataclass
class MigrationPaths:
    old_service_root: Path
    old_db: Path
    old_uploads: Path
    new_runtime_root: Path
    new_db: Path
    new_uploads: Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Migrate YT-Panel runtime data into YT-panel-Rust")
    parser.add_argument("--old-service-root", type=Path, default=DEFAULT_OLD_SERVICE_ROOT)
    parser.add_argument("--new-runtime-root", type=Path, default=DEFAULT_NEW_RUNTIME_ROOT)
    parser.add_argument("--old-db", type=Path)
    parser.add_argument("--new-db", type=Path)
    parser.add_argument("--old-uploads", type=Path)
    parser.add_argument("--new-uploads", type=Path)
    parser.add_argument("--public-user-id", type=int, default=1, help="Fallback public user id for panel_public_user_id")
    parser.add_argument("--force", action="store_true", help="Backup and overwrite an existing Rust runtime DB/uploads")
    parser.add_argument("--dry-run", action="store_true", help="Only print what would happen")
    return parser.parse_args()


def build_paths(args: argparse.Namespace) -> MigrationPaths:
    old_service_root = args.old_service_root.resolve()
    new_runtime_root = args.new_runtime_root.resolve()
    return MigrationPaths(
        old_service_root=old_service_root,
        old_db=(args.old_db.resolve() if args.old_db else old_service_root / "database" / "database.db"),
        old_uploads=(args.old_uploads.resolve() if args.old_uploads else old_service_root / "uploads"),
        new_runtime_root=new_runtime_root,
        new_db=(args.new_db.resolve() if args.new_db else new_runtime_root / "database" / "database.db"),
        new_uploads=(args.new_uploads.resolve() if args.new_uploads else new_runtime_root / "uploads"),
    )


def timestamp() -> str:
    return datetime.now().strftime("%Y%m%d-%H%M%S")


def count_files(root: Path) -> int:
    if not root.exists():
        return 0
    return sum(1 for path in root.rglob("*") if path.is_file())


def count_rows(db_path: Path, table: str) -> int:
    conn = sqlite3.connect(db_path)
    try:
        cur = conn.cursor()
        return int(cur.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0])
    finally:
        conn.close()


def list_tables(db_path: Path) -> list[str]:
    conn = sqlite3.connect(db_path)
    try:
        cur = conn.cursor()
        rows = cur.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetchall()
        return [row[0] for row in rows]
    finally:
        conn.close()


def ensure_notice_and_settings(db_path: Path, public_user_id: int) -> None:
    conn = sqlite3.connect(db_path)
    try:
        cur = conn.cursor()
        cur.execute(NOTICE_TABLE_SQL)
        cur.execute(
            "CREATE TABLE IF NOT EXISTS system_setting (id INTEGER PRIMARY KEY AUTOINCREMENT, config_name TEXT UNIQUE, config_value TEXT)"
        )
        for key, value in DEFAULT_SETTINGS.items():
            cur.execute(
                "INSERT OR IGNORE INTO system_setting (config_name, config_value) VALUES (?, ?)",
                (key, value),
            )
        cur.execute(
            "INSERT OR IGNORE INTO system_setting (config_name, config_value) VALUES (?, ?)",
            ("panel_public_user_id", str(public_user_id)),
        )
        conn.commit()
    finally:
        conn.close()


def backup_path(path: Path) -> Path:
    backup = path.with_name(f"{path.name}.bak-{timestamp()}")
    if path.is_dir():
        shutil.copytree(path, backup)
    else:
        backup.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, backup)
    return backup


def copy_uploads(src: Path, dst: Path) -> None:
    dst.mkdir(parents=True, exist_ok=True)
    if not src.exists():
        return
    for path in src.rglob("*"):
        relative = path.relative_to(src)
        target = dst / relative
        if path.is_dir():
            target.mkdir(parents=True, exist_ok=True)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)


def preflight(paths: MigrationPaths, force: bool, dry_run: bool) -> list[str]:
    messages: list[str] = []
    if not paths.old_db.exists():
        raise SystemExit(f"legacy db not found: {paths.old_db}")

    old_tables = set(list_tables(paths.old_db))
    required = {
        "bookmark",
        "file",
        "item_icon",
        "item_icon_group",
        "module_config",
        "notepad",
        "search_engine",
        "system_setting",
        "user",
        "user_config",
    }
    missing = sorted(required - old_tables)
    if missing:
        raise SystemExit(f"legacy db missing required tables: {', '.join(missing)}")

    new_db_exists = paths.new_db.exists()
    new_upload_count = count_files(paths.new_uploads)
    if new_db_exists and not force and not dry_run:
        raise SystemExit(
            f"destination db already exists: {paths.new_db}\n"
            "Refusing to overwrite. Re-run with --force after confirming backup/rollback plan."
        )
    if new_upload_count > 0 and not force and not dry_run:
        raise SystemExit(
            f"destination uploads already contain {new_upload_count} files: {paths.new_uploads}\n"
            "Refusing to merge silently. Re-run with --force after confirming backup/rollback plan."
        )

    messages.append(f"legacy db: {paths.old_db}")
    messages.append(f"legacy uploads: {paths.old_uploads} ({count_files(paths.old_uploads)} files)")
    messages.append(f"target db: {paths.new_db}{' (exists)' if new_db_exists else ''}")
    messages.append(f"target uploads: {paths.new_uploads} ({new_upload_count} files currently present)")
    return messages


def print_counts(db_path: Path, tables: Iterable[str]) -> None:
    for table in tables:
        try:
            print(f"  {table}: {count_rows(db_path, table)}")
        except sqlite3.Error as exc:
            print(f"  {table}: error ({exc})")


def main() -> int:
    args = parse_args()
    paths = build_paths(args)
    messages = preflight(paths, force=args.force, dry_run=args.dry_run)

    print("[preflight]")
    for message in messages:
        print(f"- {message}")

    if args.dry_run:
        print("\n[dry-run] no files were changed")
        return 0

    backups: list[Path] = []
    if args.force:
        if paths.new_db.exists():
            backup = backup_path(paths.new_db)
            backups.append(backup)
            print(f"[backup] db -> {backup}")
            paths.new_db.unlink()
        if paths.new_uploads.exists() and any(paths.new_uploads.iterdir()):
            backup = backup_path(paths.new_uploads)
            backups.append(backup)
            print(f"[backup] uploads -> {backup}")
            shutil.rmtree(paths.new_uploads)

    paths.new_db.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(paths.old_db, paths.new_db)
    copy_uploads(paths.old_uploads, paths.new_uploads)
    ensure_notice_and_settings(paths.new_db, args.public_user_id)

    print("\n[result]")
    print(f"- copied db -> {paths.new_db}")
    print(f"- copied uploads -> {paths.new_uploads} ({count_files(paths.new_uploads)} files)")
    if backups:
        print("- backups:")
        for backup in backups:
            print(f"  - {backup}")

    print("- destination table counts:")
    print_counts(paths.new_db, [
        "user",
        "user_config",
        "item_icon_group",
        "item_icon",
        "bookmark",
        "search_engine",
        "notepad",
        "file",
        "module_config",
        "system_setting",
        "notice",
    ])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
