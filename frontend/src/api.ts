// GNU AGPL v3 License

import axios from "axios";

import getConsts from "./consts";

// the global axios instance, with a config
const api = axios.create({
    baseURL: getConsts().api_url,
    timeout: 1000,
});

interface PaginationParameters {
    page_size: number,
    page_index: number,
}

// Combine pagination parameters with filtering options
export type ListParameters<T> = PaginationParameters & Partial<T>;

// send a GET request to retrive a list of objects, with a 
// partial filtering
export function list<T>(name: string, params: ListParameters<T>): Promise<T[]> {
    return api.get(`${name}`, { params }).then(res => res.data);
};