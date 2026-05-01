echo 'Building backend...';
# pnpm run build:backend
cargo build --release
echo 'stop service and copying backend...';
sudo systemctl stop movie-games;
# sudo cp ./server/target/release/server /srv/movie-games-server/server
sudo cp ./target/release/server /srv/movie-games-server/server
echo 'starting service...';
sudo systemctl start movie-games;
sudo systemctl status movie-games;
echo 'Successfully deployed backend!';