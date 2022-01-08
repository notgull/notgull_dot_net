// GNU AGPL v3 License

import getConsts from "./consts";

export function navlink(
    elemId: string,
    targetHref: string,
    targetText: string,
    tester: () => boolean,
    otherwise: (a: HTMLElement) => void = _ => {},
) {
    const elem = document.getElementById(elemId);
    if (elem !== null) {
         if (tester()) {
             const webUrl = getConsts().web_url;
             const linkElem = document.createElement("a");
             linkElem.href = `${webUrl}/${targetHref}`;
             linkElem.text = targetText;
             elem.appendChild(linkElem);
         } else {
             otherwise(elem);
         }
    }
};

export default navlink;