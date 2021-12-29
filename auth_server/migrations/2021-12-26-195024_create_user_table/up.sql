-- GNU AGPL v3 License

CREATE TABLE Shadow (
    id SERIAL PRIMARY KEY,
    hashed_password BYTEA NOT NULL
);

CREATE TABLE ManagedUsers (
    id SERIAL PRIMARY KEY,
    salt BYTEA NOT NULL,
    email VARCHAR NOT NULL UNIQUE,
    username VARCHAR NOT NULL UNIQUE,
    login_attempts INT NOT NULL DEFAULT 0,
    -- null if the account is not blocked
    blocked_on TIMESTAMP DEFAULT NULL,
    shadow INT NOT NULL,

    CONSTRAINT fk_shadow
      FOREIGN KEY(shadow)
        REFERENCES Shadow(id)
);

CREATE TABLE Users (
    id SERIAL PRIMARY KEY,
    uuid VARCHAR NOT NULL UNIQUE,
    managed INT REFERENCES ManagedUsers
);

CREATE TABLE IpAddresses (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    ip_address VARCHAR NOT NULL,
    last_used TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, ip_address),
    CONSTRAINT fk_user
      FOREIGN KEY(user_id)
        REFERENCES ManagedUsers(id)
)