import type { ChatLine } from "../chat";
import { ChatMessageType } from "../../client";
import { useMemo, type FC, type ReactNode } from "react";
import bbobHTML from '@bbob/html'
import presetHTML5 from '@bbob/preset-html5'
import { getCssColorForName } from "../utils/getColorForName";

import styles from './chatMessage.module.css'

interface ChatMessageProps {
    textLine: ChatLine;
}

const StylizedChatMessage: FC<{ text: string }> = ({ text }) => {
    let processed = bbobHTML(text, presetHTML5(), {
        onlyAllowTags: ["color", "b", "i", "u", "s"]
    });

    return (
        <span 
            style={{
                whiteSpace: "pre-line"
            }} 
            dangerouslySetInnerHTML={{ __html: processed }}
        />
    )
}

export const ChatMessage: FC<ChatMessageProps> = ({ textLine }) => {
    const userColor = useMemo(() => getCssColorForName(textLine.author || "anonymous"), [textLine.author]);

    console.log(userColor);
    let messageContent: ReactNode = textLine.type === ChatMessageType.stylized
        ? <StylizedChatMessage text={textLine.message} />
        : <span>
            <b style={{ color: userColor }} >{textLine.author}</b>
            {": "}
            {textLine.message}
        </span>;


    return (
        <div className={`break-words text-foreground m-1 ${styles.chatMessageAnim}`}>
            {messageContent}
        </div>
    )
}
