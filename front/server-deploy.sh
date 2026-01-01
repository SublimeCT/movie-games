echo '===========  Server Deploy  ===========';
echo 'stop service and copying backend...';
sudo systemctl stop movie-games;
sudo cp ../movie-games-server-static/target/release/server /srv/movie-games-server/server
echo 'starting service...';
sudo systemctl start movie-games;
sudo systemctl status movie-games;
echo 'Successfully deployed backend!';
echo '===========   Deploy End    ===========';