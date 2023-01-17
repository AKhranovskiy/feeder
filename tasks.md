# Tasks

## Web service

[x] Implement a web-service that takes a radio stream/playlist URL, fetch and re-stream it.

## Ads replacement

Replace detected audio with desired content.

[x] Detect ads in stream and silence audio.

[x] Replace by another audio, exact piece, no buffering
    If ads is detected, start playing the replacement audio.
    When ads ends, stop playing the replacement audio.

[ ] 1w  Cross-fade
        Add cross-fading for entering and exiting ads-replacement block.

[ ] 1w  Buffer stream while ads replacement is playing
        If ads detected, start playing the replacement audio.
        If ads ends, continue playing the replacement audio to the end,
        and buffer the original stream, skipping ads.
        When the replacement audio ends, play from buffer.

## Integration

Integrate with Ads network (D.) 

[ ] 1w  Fetch ads from AdsNet
    [ ] VAST parser
    [ ] Download ads and store them locally.
[ ] 0.5w    Play random ads from AdsNet
            Select an ads according to VAST information, so ads do not repeat endlessly.
[ ] 0.5w    Feedback to AdsNet
            Send feedback to AdsNet on playing ads.

## Training

Automate data collection and training

[ ] 2w  Revive ads scrapper for training
    [ ] Create storage
    [ ] Create standalone tool that takes HLS and scraps ads
    [ ] Create interface to review scrapped ads
[ ] 2d  Nightly job to train on confirmed data

## Detection tuning

How to improve quality of detection?

[ ] Investigate false-positives when ads is detected in the middle of song.
[ ] How to detect talks?
[ ] How to improve ads detection?

## Deployment

Deploy project to Google Cloud for training and demo.

[ ] Prepare Docker container with server and pre-trained model for Google Cloud
[ ] Prepare Docker container with scrappers and training data for Google Cloud
[ ] Train in Google Cloud

## Android

Develop Android application for demo

[ ] Make sample app for Android that use TensorFlow Lite to detect ads
