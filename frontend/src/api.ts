// GNU AGPL v3 License

import axios from "axios";

import getConsts from "./consts";

// the global axios instance, with a config
const api = axios.create({
    baseURL: getConsts().api_url,
    timeout: 1000,
});

interface PaginationParameters {
    skip: number,
    count: number,
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

export type PatchParameters<T> = Partial<T> & NoId;

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

// send a GET request to retrieve a specific object
export function get<T>(name: string, id: number): Promise<T> {
    return api.get(`${name}/${id}`, { params: authDetails() }).then(res => res.data);
};

// send a POST request to create a new object
export function post<T>(name: string, params: PostParameters<T>): Promise<number> {
    const realParams = Object.assign(params, authDetails());
    return api.post(`${name}`, params).then(res => res.data.id);
};

// send a PATCH request to update an object
export function patch<T>(name: string, id: number, params: PatchParameters<T>): Promise<void> {
    const realParams = Object.assign(params, authDetails());
    return api.patch(`${name}/${id}`, realParams).then(_ => {});
};

// send a DELETE request to delete an object
export function doDelete(name: string, id: number): Promise<void> {
    return api.delete(`${name}/${id}`, { data: authDetails() }).then(_ => {});
}

// upload a form data using POST
export function postFormData<T>(name: string, data: FormData): Promise<T> {
    return api.post(`${name}`, data).then(res => res.data);
}