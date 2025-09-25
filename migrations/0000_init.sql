-- Initial schema for core blockchain tables

-- Blocks table
CREATE TABLE IF NOT EXISTS blocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_number INTEGER NOT NULL UNIQUE,
    hash TEXT NOT NULL UNIQUE,
    previous_hash TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    block_data TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY,
    block_number INTEGER,
    transaction_type TEXT NOT NULL,
    sender TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    signature TEXT NOT NULL,
    transaction_data TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (block_number) REFERENCES blocks (block_number)
);

-- Accounts table
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    public_key TEXT NOT NULL UNIQUE,
    reputation INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    metadata TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Laws table
CREATE TABLE IF NOT EXISTS laws (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    category TEXT NOT NULL,
    status TEXT NOT NULL,
    author TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    parent_law_id TEXT,
    content_hash TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Votes table
CREATE TABLE IF NOT EXISTS votes (
    id TEXT PRIMARY KEY,
    law_id TEXT NOT NULL,
    voter TEXT NOT NULL,
    vote_type TEXT NOT NULL,
    weight REAL NOT NULL,
    timestamp TEXT NOT NULL,
    signature TEXT NOT NULL,
    comment TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (law_id) REFERENCES laws (id),
    UNIQUE(law_id, voter)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_blocks_number ON blocks (block_number);
CREATE INDEX IF NOT EXISTS idx_transactions_block ON transactions (block_number);
CREATE INDEX IF NOT EXISTS idx_votes_law ON votes (law_id);
