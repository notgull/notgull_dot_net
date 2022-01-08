// GNU AGPL v3 License

import { Component, h } from "preact";

import ListBlogpost from "./listblogpost";
import RssFeedDisplay from "./rss_display";
import { Empty } from "./util";

export function Frontpage(props: Empty) {
    return (
        <table>
            <tbody>
            <tr>
                <td>
                    <ListBlogpost />
                </td>
                <td>
                    <RssFeedDisplay />
                </td>
            </tr>
            </tbody>
        </table>
    );
};

export default Frontpage;