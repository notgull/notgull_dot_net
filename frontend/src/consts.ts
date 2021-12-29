// GNU AGPL v3 License

declare global {
    interface Window {
        constants: Consts,
    }   
}

export interface Consts {
    api_url: string,
    auth_url: string,
    csrf_token?: string,
};

const CONSTS: Consts = window.constants; 

export function getConsts(): Consts {
    return CONSTS;
};

export default getConsts;