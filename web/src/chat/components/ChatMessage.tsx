import type { ChatLine } from "../chat";
import { ChatMessageType } from "../../client";
import type { FC, ReactNode } from "react";
import bbobHTML from '@bbob/html'
import presetHTML5 from '@bbob/preset-html5'


interface ChatMessageProps {
    textLine: ChatLine;
}

const StylizedChatMessage: FC<{ text: string }> = ({ text }) => {
    let processed = bbobHTML(text, presetHTML5(), {
        onlyAllowTags: ["color", "b", "i", "u", "s"]
    });

    return <span style={{ whiteSpace: "pre-line" }} dangerouslySetInnerHTML={{ __html: processed }}></span>;
}

export const ChatMessage: FC<ChatMessageProps> = ({ textLine }) => {

    let messageContent: ReactNode = textLine.type === ChatMessageType.stylized
        ? <StylizedChatMessage text={textLine.message} />
        : textLine.message;


    return (
        <div>
            {messageContent}
        </div>
    )
}