import React from 'react';

import List from '@mui/material/List';
import Divider from '@mui/material/Divider';
import Box from '@mui/material/Box';
import ListItem from '@mui/material/ListItem';

import { TweetLi, TweetProps } from './TweetCard'

export const TweetView = ({tweets}: {tweets: Array<TweetProps>}) => {

    return (
        <List
          sx={{
            //maxWidth: 360,
            bgcolor: 'background.paper',
          }}
        >
            {
                tweets.length > 0 &&
                    tweets.map((row) => {
                        return (
                         <React.Fragment>
                            <TweetLi
                                tweet_id={row.tweet_id}
                                author_id={row.author_id}
                                username={row.username}
                                user_id={row.user_id}
                                time={row.time}
                                tweet={row.tweet}
                                profile_image_url={row.profile_image_url}
                                attachments={row.attachments}
                                focus={false}
                                />
                            <Divider component="li" />
                         </React.Fragment>
                        )
                    })
            }

            {/* Empty box */}
            <ListItem>
               <Box
                 sx={{
                   height: "calc(var(--canvas-height) - var(--appbar-height) - var(--footer-height))",
                 }}
               />
            </ListItem>
        </List>
     );
}

