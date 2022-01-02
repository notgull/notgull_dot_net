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

interface AuthDetails {
    csrf_token: string,
    csrf_cookie: string,
}

interface NoId {
    id?: never,
}

export type PostParameters<T> = T & NoId;

function authDetails(): AuthDetails {
    const consts = getConsts();
    return {
        csrf_token: consts.csrf_token!,
        csrf_cookie: consts.csrf_cookie!,
    };
}

// send a GET request to retrive a list of objects, with a 
// partial filtering
export function list<T>(name: string, params: ListParameters<T>): Promise<T[]> {
    const realParams = Object.assign(params, authDetails());
    return api.get(`${name}`, { params: realParams }).then(res => res.data);
};

// send a POST request to create a new object
export function post<T>(name: string, params: PostParameters<T>): Promise<number> {
    const realParams = Object.assign(params, authDetails());
    return api.post(`${name}`, params).then(res => res.data.id);
};