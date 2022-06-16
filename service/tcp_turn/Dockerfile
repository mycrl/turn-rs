FROM node:latest
WORKDIR /app

COPY ./services/tcp_turn/dist ./dist
COPY ./modules/bytes ./node_modules/bytes

CMD node ./dist
