echo 'Building backend...';
pnpm run build:backend
echo 'stop service and copying backend...';
sudo systemctl stop movie-games;
sudo cp ./server/target/release/server /srv/movie-games-server/server
echo 'starting service...';
sudo systemctl start movie-games;
sudo systemctl status movie-games;
echo 'Successfully deployed backend!';