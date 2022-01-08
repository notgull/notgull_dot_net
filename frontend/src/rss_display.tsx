// GNU AGPL v3 License

import { Component, Fragment, h } from "preact";

import Loading from "./loading";
import { fetchRssFeed, RssFeedEntry } from "./rss";
import { Empty, LoadingState } from "./util";

interface RssFeedState {
    entries: RssFeedEntry[],
    loadstate: LoadingState,
};

export class RssFeedDisplay extends Component<Empty, RssFeedState> {
    state = {
        entries: [],
        loadstate: LoadingState.Unmounted,
    };

    componentDidMount() {
        this.setState({
            loadstate: LoadingState.Loading,
        });

        fetchRssFeed().then(entries => {
            this.setState({
                entries,
                loadstate: LoadingState.Loaded,
            });
        });
    }

    render() {
        if (this.state.loadstate == LoadingState.Unmounted) {
            return <></>;
        } else if (this.state.loadstate == LoadingState.Loading) {
            return <Loading />;
        } else if (this.state.loadstate == LoadingState.Loaded) {
            return this.state.entries.map(entry => (
                <RssEntry entry={entry} />
            ));
        } else {
            return <></>;
        }
    }
};

interface RssEntryProps {
    entry: RssFeedEntry,
};

function RssEntry(props: RssEntryProps) {
    const { entry } = props;

    return (
        <table>
            <tr>
                <td>
                    <img src={entry.link} alt="Video thumbnail" />
                </td>
                <td>
                    <p>
                        <a href={entry.link}>{entry.title}</a>
                    </p>
                    <p>{entry.published}</p>
                </td>
            </tr>
        </table>
    );
};

export default RssFeedDisplay;