// GNU AGPL v3 License

import { h } from "preact";

interface PaginationProps {
    page: number,
    setPage: (newNum: number) => void,
};

const LEFT_ARROW: string = "←";
const RIGHT_ARROW: string = "→";

export function Pagination(props: PaginationProps) {
    const { page, setPage } = props;

    return (
        <div>
            <p>
                <a onClick={() => setPage(page - 1)}>{LEFT_ARROW}</a>
                |
                {page + 1}
                |
                <a onClick={() => setPage(page + 1)}>{RIGHT_ARROW}</a>
            </p>
        </div>
    );
};

export default Pagination;