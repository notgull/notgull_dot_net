// GNU AGPL v3 License

/// An empty dict.
export type Empty = Record<string, never>;

/// State for loading something from the API
export enum LoadingState {
    Unmounted,
    Loading,
    Loaded,
    ErrorOccurred,
};

/// Possible page sizes.
export enum PageSize {
    _10 = 10,
    _25 = 25,
    _50 = 50,
    _100 = 100,
};

/// Capitalize a string.
export function capitalize(s: string): string {
    return s.charAt(0).toUpperCase() + s.slice(1);
}