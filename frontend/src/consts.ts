// GNU AGPL v3 License

declare global {
    interface Window {
        constants: Consts,
    }   
}

export interface Consts {
    api_url: string,
    auth_url: string,
    web_url: string,
    static_url: string,
    cur_blogpost_id?: number,
    csrf_token?: string,
    csrf_cookie?: string,
    user_id?: number,
    user_perms: number,
    cur_user_id?: number,
};

const CONSTS: Consts = window.constants; 

export function getConsts(): Consts {
    return CONSTS;
};

export default getConsts;