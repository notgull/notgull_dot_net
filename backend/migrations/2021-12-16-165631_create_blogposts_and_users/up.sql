-- GNU AGPL v3 License 

CREATE TABLE Users (
  id SERIAL PRIMARY KEY,
  uuid VARCHAR NOT NULL UNIQUE,
  name VARCHAR NOT NULL,
  roles BIGINT NOT NULL
);

CREATE TABLE Blogposts (
  id SERIAL PRIMARY KEY,
  title VARCHAR NOT NULL,
  tags VARCHAR NOT NULL,
  url VARCHAR NOT NULL UNIQUE,
  body TEXT NOT NULL,
  author_id INT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  CONSTRAINT fk_author
    FOREIGN KEY(author_id)
      REFERENCES Users(id)
)
