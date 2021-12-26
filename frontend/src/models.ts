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