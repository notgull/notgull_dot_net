// GNU AGPL v3 License

// analagous to the Blogpost struct on the backend
export interface Blogpost {
    id: number,
    title: string,
    tags: string,
    url: string,
    body: string,
    author_id: number,
    created_at: Date
};

// analagous to the User struct on the backend
export interface User {
    id: number,
    uuid: string,
    name: string | undefined,
    roles: number,
};