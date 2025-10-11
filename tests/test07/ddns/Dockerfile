FROM alpine:latest

RUN apk add jq curl bind-tools tini

ENV CF_API=https://api.cloudflare.com/client/v4 
ENV RRTYPE=A 
RUN echo "@reboot /etc/cont-init.d/30-cloudflare-setup" > /etc/crontabs/root
RUN echo "*/5 * * * * /etc/cont-init.d/30-cloudflare-setup" >> /etc/crontabs/root
RUN mkdir /config

# COPY root /

ENTRYPOINT ["/sbin/tini", "--"]
# -f: Foreground, -l: Log level, -d: Log level for cron
CMD ["crond", "-f", "-l", "8", "-d", "6"]
