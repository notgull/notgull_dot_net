// GNU AGPL v3 License

import axios from "axios";

const RSS_FEED_URL = 
    "https://www.youtube.com/feeds/videos.xml?channel_id=UCa22ge_MKVapVkX8lN1jDuQ";
const RSS_FEED_COUNT = 6;

export function fetchRssFeed(): Promise<RssFeedEntry[]> {
    return axios.get(RSS_FEED_URL).then(data => (
        readRssFeed(data.data)
    ));
};

function readRssFeed(data: string): RssFeedEntry[] {
    const parser = new DOMParser();
    const xmlDoc = parser.parseFromString(data, "text/xml");

    return [].slice.call(
        xmlDoc.getElementsByClassName("entry"), 0, RSS_FEED_COUNT)
            .map(entry => new RssFeedEntry(entry)
    );
};

export class RssFeedEntry {
    public title: string;
    public published: Date;
    public link: string;
    public thumbnail: string;

    constructor(elem: Element) {
        this.title = getElemInnardsByClassName(elem, "title");
        this.published = new Date(getElemInnardsByClassName(elem, "published"));
        this.link = getAttributeOfElemByClassName(elem, "link", "href");
        this.thumbnail = getAttributeOfElemByClassName(elem, "media:thumbnail", "url");
    }
};

function getElemInnardsByClassName(base: Element, name: string): string {
    const elems = base.getElementsByClassName(name);
    const elem = elems[0];

    return elem.innerHTML;
}

function getAttributeOfElemByClassName(base: Element, name: string, attr: string): string {
    const elems = base.getElementsByClassName(name);
    const elem = elems[0];
    return elem.attributes.getNamedItem(attr)!.value;
}