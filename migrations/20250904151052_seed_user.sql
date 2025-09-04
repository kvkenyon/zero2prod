-- migrations/20250904151052_seed_user.sql
INSERT INTO users (
  user_id,
  username,
  password_hash
) VALUES (
 'ddf8994f-d522-4659-8d02-c1d479057be6',
 'admin',
 '$argon2id$v=19$m=15000,t=2,p=1$bIJBL/iptPt68apX+zs24Q$Zh5lUyly4BUlsGmjSyrc1xq4oRKngcza3yfJ/MmImSI'
);

