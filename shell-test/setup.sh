rm -f *.csv
rm -f zet.sh
echo "a1\na2\nx1" > a.csv
echo "a1\na2\na3\nx1\ny1\nz1" > a-now.csv
echo "b1\nb2\nx1" > b.csv
echo "b1\nb2\nb3\nx1\ny1\nz1" > b-now.csv
echo "c1\nc2\nx1" > c.csv
echo "c1\nc2\nc3\nx1\ny1\nz1" > c-now.csv
rg '\$ (zet.*)' --replace '$1' ../README.md > zet.sh
